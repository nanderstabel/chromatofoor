use crate::generator::BoundedContextGenerator;
use crate::schema_utils::generate_type_for_prop;
use anyhow::Result;
use convert_case::Casing;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_application(
    out_dir: &PathBuf,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    contexts: &HashMap<(String, String), crate::generator::GeneratorContext>,
    routing: &crate::generator::RoutingContext,
) -> Result<()> {
    // Collect all BCs from both bc_contexts and contexts
    let mut all_bcs: std::collections::HashSet<String> = bc_contexts.keys().cloned().collect();
    for (bc, _) in contexts.keys() {
        all_bcs.insert(bc.clone());
    }
    let mut sorted_bcs: Vec<String> = all_bcs.into_iter().collect();
    sorted_bcs.sort();

    for bc in sorted_bcs {
        if bc == "global" {
            continue;
        }
        let bc_ctx = bc_contexts.get(&bc);

        let bc_dir = crate::generator::resolve_bc_dir(out_dir, &bc, routing);
        let src_dir = bc_dir.join("src");
        let app_dir = src_dir.join("generated").join("application");
        let manual_app_dir = src_dir.join("application");

        fs::create_dir_all(&app_dir)?;
        fs::create_dir_all(&manual_app_dir)?;
        let mut app_mod_chunks = Vec::new();

        // --- Generate Read Models ---
        let empty_read_models = Vec::new();
        let empty_queries = Vec::new();
        let read_models = bc_ctx.map_or(&empty_read_models, |ctx| &ctx.read_models);
        let queries = bc_ctx.map_or(&empty_queries, |ctx| &ctx.queries);

        if !read_models.is_empty() {
            let rm_dir = app_dir.join("read_models");
            fs::create_dir_all(&rm_dir)?;
            let mut rm_mod_chunks = Vec::new();

            let manual_rm_dir = manual_app_dir.join("read_models");
            fs::create_dir_all(&manual_rm_dir)?;
            let mut manual_rm_mods = Vec::new();

            for rm in read_models {
                let rm_name = &rm.name; // e.g. "apartment-inventory"
                let rm_snake = rm_name.to_case(convert_case::Case::Snake);
                let rm_pascal = rm_name.to_case(convert_case::Case::Pascal);

                let rm_ident = format_ident!("{}", rm_pascal);
                let rm_record_ident = format_ident!("{}Record", rm_pascal);

                let mut projected_events: Vec<(String, String)> = Vec::new();
                let mut seen_projected = std::collections::HashSet::new();
                let mut sorted_ctx_keys: Vec<_> = contexts.keys().collect();
                sorted_ctx_keys.sort();

                if let Some(projections) = &rm.projections {
                    for proj in projections {
                        if proj
                            .bounded_context
                            .as_deref()
                            .is_some_and(|p_bc| p_bc != bc)
                        {
                            continue;
                        }
                        let Some(ev_name) = &proj.event else {
                            continue;
                        };
                        if let Some(agg) = &proj.aggregate {
                            if agg == "Unknown" {
                                continue;
                            }
                            let is_known = if contexts.is_empty() {
                                true
                            } else {
                                contexts.get(&(bc.clone(), agg.clone())).is_some_and(|ctx| {
                                    ctx.events.iter().any(|e| &e.name == ev_name)
                                })
                            };
                            if is_known && seen_projected.insert((agg.clone(), ev_name.clone())) {
                                projected_events.push((agg.clone(), ev_name.clone()));
                            }
                        } else {
                            for (c_bc, c_unit) in &sorted_ctx_keys {
                                if *c_bc == bc
                                    && *c_unit != "global"
                                    && let Some(ctx) =
                                        contexts.get(&((*c_bc).clone(), (*c_unit).clone()))
                                    && ctx.events.iter().any(|e| &e.name == ev_name)
                                {
                                    if seen_projected.insert(((*c_unit).clone(), ev_name.clone())) {
                                        projected_events.push(((*c_unit).clone(), ev_name.clone()));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }

                let mut type_ts = quote! { {} };
                let mut extra_structs = Vec::new();
                if let Some(schema) = &rm.schema {
                    let mut props = Vec::new();
                    for (prop_name, prop_schema) in &schema.properties {
                        let prop_ident =
                            format_ident!("{}", prop_name.to_case(convert_case::Case::Snake));
                        let is_required = schema.required.contains(prop_name);

                        let (prop_ts, mut e_structs) =
                            generate_type_for_prop(prop_name, prop_schema, &rm_pascal);
                        extra_structs.append(&mut e_structs);

                        if is_required {
                            props.push(quote! { pub #prop_ident: #prop_ts });
                        } else {
                            props.push(quote! { pub #prop_ident: Option<#prop_ts> });
                        }
                    }

                    type_ts = quote! {
                        {
                            #(#props),*
                        }
                    };
                }

                let mut rm_chunks = Vec::new();
                rm_chunks.push(quote! {
                    #[allow(unused_imports)]
                    use chromatofoor::validator::Validate;
                    use serde::{Deserialize, Serialize};
                });

                for est in extra_structs {
                    rm_chunks.push(est);
                }

                let rm_doc = crate::generator::make_doc_comment(rm.description.as_deref());
                let bc_pascal = format_ident!("{}", bc.to_case(convert_case::Case::Pascal));

                let projected_event_paths: Vec<_> = projected_events
                    .iter()
                    .map(|(owner_unit, ev_name)| {
                        let owner_snake =
                            format_ident!("{}", owner_unit.to_case(convert_case::Case::Snake));
                        let ev_mod =
                            format_ident!("{}", ev_name.to_case(convert_case::Case::Snake));
                        let ev_struct =
                            format_ident!("{}", ev_name.to_case(convert_case::Case::Pascal));
                        quote! {
                            crate::generated::domain::#owner_snake::event::#ev_mod::#ev_struct
                        }
                    })
                    .collect();

                rm_chunks.push(quote! {
                    #rm_doc
                    #[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, chromatofoor::architecturekit::Schema)]
                    #[architecturekit(crate = "chromatofoor::architecturekit")]
                    pub struct #rm_record_ident #type_ts

                    #rm_doc
                    #[chromatofoor::read_model(
                        scope = crate::generated::domain::#bc_pascal,
                        events = [#(#projected_event_paths),*],
                    )]
                    #[derive(Default, Debug, Serialize)]
                    #[serde(transparent)]
                    pub struct #rm_ident {
                        pub records: chromatofoor::bounded_context::DocumentView<#rm_record_ident>,
                    }

                    impl #rm_ident {
                        #[must_use]
                        pub fn new() -> Self {
                            Self::default()
                        }
                    }
                });

                let manual_rm_file = manual_rm_dir.join(format!("{rm_snake}.rs"));
                let rm_snake_ident = format_ident!("{}", rm_snake);
                if !manual_rm_file.exists() {
                    let manual_rm_impls: Vec<_> = projected_event_paths
                        .iter()
                        .map(|ev_path| {
                            quote! {
                                impl Project<#ev_path> for #rm_ident {
                                    async fn project(
                                        &self,
                                        _event: Envelope<#ev_path>,
                                    ) -> Result<(), ViewError> {
                                        Ok(())
                                    }
                                }
                            }
                        })
                        .collect();

                    let manual_tokens = if manual_rm_impls.is_empty() {
                        quote! {
                            #[allow(unused_imports)]
                            use crate::generated::application::read_models::#rm_snake_ident::{#rm_ident, #rm_record_ident};
                        }
                    } else {
                        quote! {
                            #[allow(unused_imports)]
                            use crate::generated::application::read_models::#rm_snake_ident::{#rm_ident, #rm_record_ident};
                            use chromatofoor::{
                                architecturekit::{Envelope, ViewError},
                                bounded_context::Project,
                            };

                            #(#manual_rm_impls)*
                        }
                    };
                    crate::generator::write_manual_rust_file(&manual_rm_file, manual_tokens)?;
                }
                let rm_tokens = quote! { #(#rm_chunks)* };
                crate::generator::write_rust_file(
                    &rm_dir.join(format!("{rm_snake}.rs")),
                    rm_tokens,
                )?;
                rm_mod_chunks.push(quote! { pub mod #rm_snake_ident; });
                manual_rm_mods.push(rm_snake);
            }

            let rm_mod_tokens = quote! { #(#rm_mod_chunks)* };
            crate::generator::write_rust_file(&rm_dir.join("mod.rs"), rm_mod_tokens)?;
            app_mod_chunks.push(quote! { pub mod read_models; });

            let manual_rm_mod = manual_rm_dir.join("mod.rs");
            if !manual_rm_mods.is_empty() {
                crate::generator::ensure_manual_mod_file(&manual_rm_mod, &manual_rm_mods)?;
            }
        }

        // --- Generate Queries ---
        if !queries.is_empty() {
            let query_dir = app_dir.join("query");
            fs::create_dir_all(&query_dir)?;
            let mut query_mod_chunks = Vec::new();

            for q in queries {
                let q_name = &q.name;
                let q_snake = q_name.to_case(convert_case::Case::Snake);
                let q_pascal = q_name.to_case(convert_case::Case::Pascal);

                let query_ident = format_ident!("{}", q_pascal);

                let mut q_chunks = Vec::new();

                let mut is_list = false;
                if let Some(res) = &q.result
                    && res.prop_type.as_deref() == Some("array")
                {
                    is_list = true;
                }

                let rm_name = q.read_model.as_deref().unwrap_or("Unknown");
                let rm_snake = rm_name.to_case(convert_case::Case::Snake);
                let rm_pascal = rm_name.to_case(convert_case::Case::Pascal);

                let rm_ident = format_ident!("{}", rm_pascal);
                let rm_record_ident = format_ident!("{}Record", rm_pascal);
                let rm_mod = format_ident!("{}", rm_snake);

                let uses = vec![
                    quote! { use crate::generated::application::read_models::#rm_mod::{#rm_ident, #rm_record_ident}; },
                    quote! { use chromatofoor::bounded_context::Query; },
                ];

                let mut type_ts = quote! { {} };
                let mut query_has_id = false;
                let mut query_id_is_option = false;

                if let Some(params) = &q.parameters {
                    let mut props = Vec::new();
                    for (prop_name, prop_schema) in &params.properties {
                        let prop_ident =
                            format_ident!("{}", prop_name.to_case(convert_case::Case::Snake));
                        let (prop_ts, _) =
                            generate_type_for_prop(prop_name, prop_schema, &q_pascal);
                        let is_required = params.required.contains(prop_name);
                        if prop_name == "id" {
                            query_has_id = true;
                            query_id_is_option = !is_required;
                        }

                        if is_required {
                            props.push(quote! { pub #prop_ident: #prop_ts });
                        } else {
                            props.push(quote! { pub #prop_ident: Option<#prop_ts> });
                        }
                    }
                    if !props.is_empty() {
                        type_ts = quote! {
                            {
                                #(#props),*
                            }
                        };
                    }
                }

                q_chunks.push(quote! { #(#uses)* });

                let q_doc = crate::generator::make_doc_comment(q.description.as_deref());
                let bc_pascal = format_ident!("{}", bc.to_case(convert_case::Case::Pascal));

                let mut actor_paths = Vec::new();
                if let Some(actors) = &q.actors {
                    let bc_actors = bc_ctx.map(|b| &b.actors);
                    for a in actors {
                        let exists_in_bc =
                            bc_actors.is_some_and(|list| list.iter().any(|doc| &doc.name == a));
                        if !exists_in_bc {
                            anyhow::bail!(
                                "Query '{}' in bounded context '{}' references unknown actor '{}'",
                                q.name,
                                bc,
                                a
                            );
                        }
                        let actor_pascal =
                            format_ident!("{}", a.to_case(convert_case::Case::Pascal));
                        actor_paths.push(quote! {
                            crate::generated::domain::actors::#actor_pascal
                        });
                    }
                }

                let query_attr = if actor_paths.is_empty() {
                    quote! {
                        #[chromatofoor::query(scope = crate::generated::domain::#bc_pascal)]
                    }
                } else {
                    quote! {
                        #[chromatofoor::query(
                            scope = crate::generated::domain::#bc_pascal,
                            actors = [#(#actor_paths),*],
                        )]
                    }
                };

                q_chunks.push(quote! {
                    #q_doc
                    #query_attr
                    #[derive(Default)]
                    pub struct #query_ident #type_ts
                });

                let (query_rm_type, execute_body) = if is_list {
                    (
                        quote! { Vec<#rm_record_ident> },
                        quote! {
                            Ok(read_model.records.all().await)
                        },
                    )
                } else if query_has_id && !query_id_is_option {
                    (
                        quote! { #rm_record_ident },
                        quote! {
                            let id_str = self.id.to_string();
                            read_model
                                .records
                                .get(&id_str)
                                .await
                                .ok_or_else(|| "Item not found or deleted".to_string())
                        },
                    )
                } else if query_has_id && query_id_is_option {
                    (
                        quote! { #rm_record_ident },
                        quote! {
                            if let Some(id) = &self.id {
                                let id_str = id.to_string();
                                read_model
                                    .records
                                    .get(&id_str)
                                    .await
                                    .ok_or_else(|| "Item not found or deleted".to_string())
                            } else {
                                Ok(read_model.records.latest().await)
                            }
                        },
                    )
                } else {
                    (
                        quote! { #rm_record_ident },
                        quote! {
                            Ok(read_model.records.latest().await)
                        },
                    )
                };

                q_chunks.push(quote! {
                    impl Query<#rm_ident> for #query_ident {
                        type Output = #query_rm_type;
                        type Error = String;

                        async fn execute(
                            self,
                            read_model: &#rm_ident,
                        ) -> Result<Self::Output, Self::Error> {
                            #execute_body
                        }
                    }
                });

                let q_tokens = quote! { #(#q_chunks)* };
                crate::generator::write_rust_file(
                    &query_dir.join(format!("{q_snake}.rs")),
                    q_tokens,
                )?;
                let q_snake_ident = format_ident!("{}", q_snake);
                query_mod_chunks.push(quote! { pub mod #q_snake_ident; });
            }

            let query_mod_tokens = quote! { #(#query_mod_chunks)* };
            crate::generator::write_rust_file(&query_dir.join("mod.rs"), query_mod_tokens)?;
            app_mod_chunks.push(quote! { pub mod query; });
        }

        let app_mod_tokens = quote! { #(#app_mod_chunks)* };
        crate::generator::write_rust_file(&app_dir.join("mod.rs"), app_mod_tokens)?;

        let manual_app_mod = manual_app_dir.join("mod.rs");
        let manual_app_mods: Vec<String> = if read_models.is_empty() {
            vec![]
        } else {
            vec!["read_models".to_string()]
        };
        crate::generator::ensure_manual_mod_file(&manual_app_mod, &manual_app_mods)?;
    }

    // Run rustfmt on the generated directory

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::BoundedContextGenerator;
    use crate::model::*;
    use insta::assert_snapshot;

    #[test]
    fn test_application_generation() {
        let mut bc_contexts = std::collections::HashMap::new();
        bc_contexts.insert(
            "TestContext".to_string(),
            BoundedContextGenerator {
                features: vec![],
                entities: vec![],
                value_objects: vec![],
                domain_services: vec![],
                actors: vec![],
                context_mappings: vec![],
                external_systems: vec![],
                read_models: vec![EsdmDoc {
                    name: "TestReadModel".to_string(),
                    kind: "read-model".to_string(),
                    schema: Some(DataSchema {
                        properties: indexmap::IndexMap::from([(
                            "id".to_string(),
                            PropertySchema {
                                prop_type: Some("string".to_string()),
                                format: Some("uuid".to_string()),
                                ..Default::default()
                            },
                        )]),
                        required: vec!["id".to_string()],
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                queries: vec![],
            },
        );

        let contexts = std::collections::HashMap::new();

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &crate::generator::RoutingContext::default();
        generate_application(&out_dir, &bc_contexts, &contexts, routing).unwrap();

        let test_bc_dir = crate::generator::resolve_bc_dir(&out_dir, "TestContext", routing);
        let rm_rs = std::fs::read_to_string(
            test_bc_dir.join("src/generated/application/read_models/test_read_model.rs"),
        )
        .unwrap_or_default();

        assert_snapshot!("application_generation", rm_rs);
    }

    #[test]
    fn test_read_model_with_unknown_and_cross_bc_projections() {
        let mut bc_contexts = std::collections::HashMap::new();
        bc_contexts.insert(
            "sales".to_string(),
            BoundedContextGenerator {
                read_models: vec![EsdmDoc {
                    name: "order-summary".to_string(),
                    kind: "read-model".to_string(),
                    projections: Some(vec![
                        Projection {
                            bounded_context: Some("billing".to_string()),
                            aggregate: Some("order".to_string()),
                            event: Some("invoice-paid".to_string()),
                            rule: None,
                        },
                        Projection {
                            bounded_context: Some("sales".to_string()),
                            aggregate: Some("non-existent".to_string()),
                            event: Some("ghost-event".to_string()),
                            rule: None,
                        },
                    ]),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut contexts = std::collections::HashMap::new();
        contexts.insert(
            ("sales".to_string(), "order".to_string()),
            crate::generator::GeneratorContext::default(),
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &crate::generator::RoutingContext::default();
        generate_application(&out_dir, &bc_contexts, &contexts, routing).unwrap();

        let sales_dir = crate::generator::resolve_bc_dir(&out_dir, "sales", routing);
        let gen_rm = std::fs::read_to_string(
            sales_dir.join("src/generated/application/read_models/order_summary.rs"),
        )
        .unwrap();
        let manual_rm =
            std::fs::read_to_string(sales_dir.join("src/application/read_models/order_summary.rs"))
                .unwrap();

        assert!(gen_rm.contains("events = []"));
        assert!(!manual_rm.contains("impl Project"));
        assert!(syn::parse_file(&gen_rm).is_ok());
        assert!(syn::parse_file(&manual_rm).is_ok());
    }
}
