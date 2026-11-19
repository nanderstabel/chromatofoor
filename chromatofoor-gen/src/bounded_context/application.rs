use crate::common::{
    BoundedContextGenerator, GeneratorContext, RoutingContext, collect_schema_ref_idents,
    emit_domain_ref_imports, ensure_manual_mod_file, generate_type_for_prop, make_doc_comment,
    make_doc_lines, resolve_bc_dir, write_manual_rust_file, write_rust_file,
};
use anyhow::Result;
use convert_case::Casing;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_application(
    out_dir: &PathBuf,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    contexts: &HashMap<(String, String), GeneratorContext>,
    routing: &RoutingContext,
) -> Result<()> {
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

        let bc_dir = resolve_bc_dir(out_dir, &bc, routing);
        let src_dir = bc_dir.join("src");
        let app_dir = src_dir.join("generated").join("application");
        let manual_app_dir = src_dir.join("application");

        fs::create_dir_all(&app_dir)?;
        fs::create_dir_all(&manual_app_dir)?;
        let mut app_mod_chunks = Vec::new();

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
                let rm_name = &rm.name;
                let rm_snake = rm_name.to_case(convert_case::Case::Snake);
                let rm_pascal = rm_name.to_case(convert_case::Case::Pascal);

                let rm_ident = format_ident!("{}", rm_pascal);
                let rm_record_ident = format_ident!("{}Record", rm_pascal);

                let mut projected_events: Vec<(String, String, Option<String>)> = Vec::new();
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
                                projected_events.push((
                                    agg.clone(),
                                    ev_name.clone(),
                                    proj.rule.clone(),
                                ));
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
                                        projected_events.push((
                                            (*c_unit).clone(),
                                            ev_name.clone(),
                                            proj.rule.clone(),
                                        ));
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }

                let mut type_ts = quote! { {} };
                let mut extra_structs = Vec::new();
                let mut rm_ref_idents = std::collections::BTreeSet::new();
                if let Some(schema) = &rm.schema {
                    rm_ref_idents = collect_schema_ref_idents(schema);
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

                let rm_doc = make_doc_comment(rm.description.as_deref());
                let bc_pascal_str = bc.to_case(convert_case::Case::Pascal);
                let bc_pascal = format_ident!("{}", bc_pascal_str);
                let rm_record_str = rm_record_ident.to_string();

                let mut ev_pascal_counts: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for (_, ev_name, _) in &projected_events {
                    *ev_pascal_counts
                        .entry(ev_name.to_case(convert_case::Case::Pascal))
                        .or_insert(0) += 1;
                }

                let mut rm_event_imports = std::collections::BTreeSet::new();
                let projected_event_entries: Vec<(
                    String,
                    String,
                    Option<String>,
                    syn::Ident,
                )> = projected_events
                    .iter()
                    .map(|(owner_unit, ev_name, proj_rule)| {
                        let owner_snake =
                            format_ident!("{}", owner_unit.to_case(convert_case::Case::Snake));
                        let ev_pascal_str = ev_name.to_case(convert_case::Case::Pascal);
                        let ev_struct = format_ident!("{}", ev_pascal_str);
                        let collides = ev_pascal_counts.get(&ev_pascal_str).copied().unwrap_or(0)
                            > 1
                            || ev_pascal_str == rm_pascal
                            || ev_pascal_str == rm_record_str
                            || ev_pascal_str == bc_pascal_str
                            || rm_ref_idents.contains(&ev_pascal_str);
                        let local_ident = if collides {
                            let alias = format_ident!(
                                "{}{}",
                                owner_unit.to_case(convert_case::Case::Pascal),
                                ev_pascal_str
                            );
                            rm_event_imports.insert(
                                quote! {
                                    use crate::generated::domain::#owner_snake::event::#ev_struct as #alias;
                                }
                                .to_string(),
                            );
                            alias
                        } else {
                            rm_event_imports.insert(
                                quote! {
                                    use crate::generated::domain::#owner_snake::event::#ev_struct;
                                }
                                .to_string(),
                            );
                            ev_struct
                        };
                        (
                            ev_name.clone(),
                            ev_pascal_str,
                            proj_rule.clone(),
                            local_ident,
                        )
                    })
                    .collect();

                let projected_event_idents: Vec<_> = projected_event_entries
                    .iter()
                    .map(|(_, _, _, ident)| ident)
                    .collect();
                let parsed_event_imports: Vec<proc_macro2::TokenStream> = rm_event_imports
                    .iter()
                    .filter_map(|s| s.parse().ok())
                    .collect();

                let (bc_scope_import, bc_scope_ref) = if bc_pascal_str == rm_pascal
                    || bc_pascal_str == rm_record_str
                    || rm_ref_idents.contains(&bc_pascal_str)
                {
                    let alias = format_ident!("{}Scope", bc_pascal);
                    (
                        quote! { use crate::generated::domain::#bc_pascal as #alias; },
                        quote! { #alias },
                    )
                } else {
                    (
                        quote! { use crate::generated::domain::#bc_pascal; },
                        quote! { #bc_pascal },
                    )
                };
                let rm_ref_imports =
                    emit_domain_ref_imports(&rm_ref_idents, &[&rm_pascal, &rm_record_str]);

                rm_chunks.push(quote! {
                    #bc_scope_import
                    #rm_ref_imports
                    #(#parsed_event_imports)*
                    #[allow(unused_imports)]
                    use chromatofoor::validator::Validate;
                    use serde::{Deserialize, Serialize};
                });

                for est in extra_structs {
                    rm_chunks.push(est);
                }

                rm_chunks.push(quote! {
                    #rm_doc
                    #[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, chromatofoor::Schema)]
                    pub struct #rm_record_ident #type_ts

                    #rm_doc
                    #[chromatofoor::read_model(
                        scope = #bc_scope_ref,
                        events = [#(#projected_event_idents),*],
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
                    let rm_header = match rm.description.as_deref().map(str::trim) {
                        Some(d) if !d.is_empty() => format!("Read Model: `{rm_name}` — {d}"),
                        _ => format!("Read Model: `{rm_name}`"),
                    };
                    let manual_rm_impls: Vec<_> = projected_event_entries
                        .iter()
                        .map(|(ev_kebab, ev_pascal_str, proj_rule, ev_local_ident)| {
                            let mut doc_lines =
                                vec![rm_header.clone(), format!("Projection: `{ev_kebab}`")];
                            if let Some(rule) = proj_rule.as_deref().map(str::trim)
                                && !rule.is_empty()
                            {
                                doc_lines.push(String::new());
                                doc_lines.push(format!("Rule: {rule}"));
                            }
                            let proj_doc = make_doc_lines(&doc_lines);
                            let todo_msg = format!(
                                "implement Project::{ev_pascal_str}::project for {rm_pascal}"
                            );
                            quote! {
                                #proj_doc
                                impl Project<#ev_local_ident> for #rm_ident {
                                    async fn project(
                                        &self,
                                        _event: Envelope<#ev_local_ident>,
                                    ) -> Result<(), ViewError> {
                                        todo!(#todo_msg)
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
                            #(#parsed_event_imports)*
                            use chromatofoor::{
                                architecturekit::{Envelope, ViewError},
                                bounded_context::Project,
                            };

                            #(#manual_rm_impls)*
                        }
                    };
                    write_manual_rust_file(&manual_rm_file, manual_tokens)?;
                }
                let rm_tokens = quote! { #(#rm_chunks)* };
                write_rust_file(&rm_dir.join(format!("{rm_snake}.rs")), rm_tokens)?;
                rm_mod_chunks.push(quote! { pub mod #rm_snake_ident; });
                manual_rm_mods.push(rm_snake);
            }

            let rm_mod_tokens = quote! { #(#rm_mod_chunks)* };
            write_rust_file(&rm_dir.join("mod.rs"), rm_mod_tokens)?;
            app_mod_chunks.push(quote! { pub mod read_models; });

            let manual_rm_mod = manual_rm_dir.join("mod.rs");
            if !manual_rm_mods.is_empty() {
                ensure_manual_mod_file(&manual_rm_mod, &manual_rm_mods)?;
            }
        }

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
                    quote! { use chromatofoor::{ProblemDetails, bounded_context::Query}; },
                ];

                let mut type_ts = quote! { {} };
                let mut lookup_key: Option<(syn::Ident, bool)> = None;
                let mut q_ref_idents = std::collections::BTreeSet::new();
                let mut q_extra_structs = Vec::new();
                let mut has_required_params = false;

                if let Some(params) = &q.parameters {
                    q_ref_idents = collect_schema_ref_idents(params);
                    has_required_params = !params.required.is_empty();
                    if let Some((key_name, key_is_opt)) = detect_query_lookup_key(params) {
                        lookup_key = Some((
                            format_ident!("{}", key_name.to_case(convert_case::Case::Snake)),
                            key_is_opt,
                        ));
                    }
                    let mut props = Vec::new();
                    for (prop_name, prop_schema) in &params.properties {
                        let prop_snake = prop_name.to_case(convert_case::Case::Snake);
                        let prop_ident = format_ident!("{}", prop_snake);
                        let (prop_ts, mut e_structs) =
                            generate_type_for_prop(prop_name, prop_schema, &q_pascal);
                        q_extra_structs.append(&mut e_structs);
                        let is_required = params.required.contains(prop_name);
                        let rename_attr = if prop_snake == *prop_name {
                            quote! {}
                        } else {
                            quote! { #[serde(rename = #prop_name)] }
                        };

                        if is_required {
                            props.push(quote! {
                                #rename_attr
                                pub #prop_ident: #prop_ts
                            });
                        } else {
                            props.push(quote! {
                                #rename_attr
                                pub #prop_ident: Option<#prop_ts>
                            });
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

                let q_doc = make_doc_comment(q.description.as_deref());
                let bc_pascal_str = bc.to_case(convert_case::Case::Pascal);
                let bc_pascal = format_ident!("{}", bc_pascal_str);
                let rm_record_str = rm_record_ident.to_string();

                let mut actor_idents = Vec::new();
                let mut sorted_actor_names = std::collections::BTreeSet::new();
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
                        let actor_pascal_str = a.to_case(convert_case::Case::Pascal);
                        sorted_actor_names.insert(actor_pascal_str.clone());
                        actor_idents.push(format_ident!("{}", actor_pascal_str));
                    }
                }

                let (bc_scope_import, bc_scope_ref) = if bc_pascal_str == q_pascal
                    || bc_pascal_str == rm_pascal
                    || bc_pascal_str == rm_record_str
                    || sorted_actor_names.contains(&bc_pascal_str)
                    || q_ref_idents.contains(&bc_pascal_str)
                {
                    let alias = format_ident!("{}Scope", bc_pascal);
                    (
                        quote! { use crate::generated::domain::#bc_pascal as #alias; },
                        quote! { #alias },
                    )
                } else {
                    (
                        quote! { use crate::generated::domain::#bc_pascal; },
                        quote! { #bc_pascal },
                    )
                };

                let actor_imports = if sorted_actor_names.is_empty() {
                    quote! {}
                } else if sorted_actor_names.len() == 1 {
                    let single = format_ident!(
                        "{}",
                        sorted_actor_names.iter().next().map_or("", String::as_str)
                    );
                    quote! { use crate::generated::domain::actors::#single; }
                } else {
                    let list: Vec<_> = sorted_actor_names
                        .iter()
                        .map(|n| format_ident!("{n}"))
                        .collect();
                    quote! { use crate::generated::domain::actors::{#(#list),*}; }
                };

                let q_ref_imports = emit_domain_ref_imports(
                    &q_ref_idents,
                    &[&q_pascal, &rm_pascal, &rm_record_str],
                );

                let extra_serde_import = if q_extra_structs.is_empty() {
                    quote! {}
                } else {
                    quote! { use serde::{Deserialize, Serialize}; }
                };

                q_chunks.push(quote! {
                    #(#uses)*
                    #bc_scope_import
                    #actor_imports
                    #q_ref_imports
                    #extra_serde_import
                });

                for est in q_extra_structs {
                    q_chunks.push(est);
                }

                let query_attr = if actor_idents.is_empty() {
                    quote! {
                        #[chromatofoor::query(scope = #bc_scope_ref)]
                    }
                } else {
                    quote! {
                        #[chromatofoor::query(
                            scope = #bc_scope_ref,
                            actors = [#(#actor_idents),*],
                        )]
                    }
                };

                let default_derive = if has_required_params {
                    quote! {}
                } else {
                    quote! { #[derive(Default)] }
                };

                q_chunks.push(quote! {
                    #q_doc
                    #query_attr
                    #default_derive
                    pub struct #query_ident #type_ts
                });

                let (query_rm_type, execute_body) = if is_list {
                    (
                        quote! { Vec<#rm_record_ident> },
                        quote! {
                            Ok(read_model.records.all().await)
                        },
                    )
                } else if let Some((key_ident, false)) = &lookup_key {
                    (
                        quote! { #rm_record_ident },
                        quote! {
                            let key = self.#key_ident.to_string();
                            read_model
                                .records
                                .get(&key)
                                .await
                                .ok_or_else(|| {
                                    Box::new(ProblemDetails::not_found(format!(
                                        "{} '{key}' not found",
                                        #rm_name
                                    )))
                                })
                        },
                    )
                } else if let Some((key_ident, true)) = &lookup_key {
                    (
                        quote! { #rm_record_ident },
                        quote! {
                            if let Some(key_val) = &self.#key_ident {
                                let key = key_val.to_string();
                                read_model
                                    .records
                                    .get(&key)
                                    .await
                                    .ok_or_else(|| {
                                        Box::new(ProblemDetails::not_found(format!(
                                            "{} '{key}' not found",
                                            #rm_name
                                        )))
                                    })
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
                        type Error = Box<ProblemDetails>;

                        async fn execute(
                            self,
                            read_model: &#rm_ident,
                        ) -> Result<Self::Output, Self::Error> {
                            #execute_body
                        }
                    }
                });

                let q_tokens = quote! { #(#q_chunks)* };
                write_rust_file(&query_dir.join(format!("{q_snake}.rs")), q_tokens)?;
                let q_snake_ident = format_ident!("{}", q_snake);
                query_mod_chunks.push(quote! { pub mod #q_snake_ident; });
            }

            let query_mod_tokens = quote! { #(#query_mod_chunks)* };
            write_rust_file(&query_dir.join("mod.rs"), query_mod_tokens)?;
            app_mod_chunks.push(quote! { pub mod query; });
        }

        let app_mod_tokens = quote! { #(#app_mod_chunks)* };
        write_rust_file(&app_dir.join("mod.rs"), app_mod_tokens)?;

        let manual_app_mod = manual_app_dir.join("mod.rs");
        let manual_app_mods: Vec<String> = if read_models.is_empty() {
            vec![]
        } else {
            vec!["read_models".to_string()]
        };
        ensure_manual_mod_file(&manual_app_mod, &manual_app_mods)?;
    }

    Ok(())
}

fn detect_query_lookup_key(params: &esdm::DataSchema) -> Option<(String, bool)> {
    if params.properties.contains_key("id") {
        let is_opt = !params.required.iter().any(|r| r == "id");
        return Some(("id".to_string(), is_opt));
    }

    let id_props: Vec<&String> = params
        .properties
        .keys()
        .filter(|k| k.ends_with("-id") || k.ends_with("_id"))
        .collect();
    if id_props.len() == 1 {
        let name = id_props[0].clone();
        let is_opt = !params.required.contains(&name);
        return Some((name, is_opt));
    }

    if params.properties.len() == 1
        && let Some((name, schema)) = params.properties.iter().next()
        && matches!(schema.prop_type.as_deref(), Some("string" | "integer"))
        && schema.enum_values.is_none()
    {
        let is_opt = !params.required.contains(name);
        return Some((name.clone(), is_opt));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{BoundedContextGenerator, GeneratorContext, RoutingContext};
    use esdm::*;
    use insta::assert_snapshot;

    #[test]
    fn test_application_generation() {
        let mut bc_contexts = std::collections::HashMap::new();
        bc_contexts.insert(
            "TestContext".to_string(),
            BoundedContextGenerator {
                description: None,
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
                    projections: Some(vec![Projection {
                        bounded_context: Some("TestContext".to_string()),
                        aggregate: Some("test-agg".to_string()),
                        event: Some("test-created".to_string()),
                        rule: None,
                    }]),
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
        let routing = &RoutingContext::default();
        generate_application(&out_dir, &bc_contexts, &contexts, routing).unwrap();

        let test_bc_dir = resolve_bc_dir(&out_dir, "TestContext", routing);
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
                            bounded_context: Some("sales".to_string()),
                            aggregate: Some("order".to_string()),
                            event: Some("order-placed".to_string()),
                            rule: None,
                        },
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
            GeneratorContext {
                events: vec![EsdmDoc {
                    name: "order-placed".to_string(),
                    kind: "event".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &RoutingContext::default();
        generate_application(&out_dir, &bc_contexts, &contexts, routing).unwrap();

        let sales_dir = resolve_bc_dir(&out_dir, "sales", routing);
        let gen_rm = std::fs::read_to_string(
            sales_dir.join("src/generated/application/read_models/order_summary.rs"),
        )
        .unwrap();
        let manual_rm =
            std::fs::read_to_string(sales_dir.join("src/application/read_models/order_summary.rs"))
                .unwrap();

        assert!(gen_rm.contains("events = [OrderPlaced]"));
        assert!(!gen_rm.contains("InvoicePaid"));
        assert!(!gen_rm.contains("GhostEvent"));
        assert!(manual_rm.contains("impl Project<OrderPlaced>"));
        assert!(!manual_rm.contains("InvoicePaid"));
        assert!(syn::parse_file(&gen_rm).is_ok());
        assert!(syn::parse_file(&manual_rm).is_ok());
    }

    #[test]
    fn test_query_key_lookup_and_list_generation() {
        let mut bc_contexts = std::collections::HashMap::new();
        bc_contexts.insert(
            "enrollment".to_string(),
            BoundedContextGenerator {
                actors: vec![EsdmDoc {
                    name: "student".to_string(),
                    kind: "actor".to_string(),
                    ..Default::default()
                }],
                read_models: vec![EsdmDoc {
                    name: "course-roster".to_string(),
                    kind: "read-model".to_string(),
                    ..Default::default()
                }],
                queries: vec![
                    EsdmDoc {
                        name: "get-course-roster".to_string(),
                        kind: "query".to_string(),
                        read_model: Some("course-roster".to_string()),
                        actors: Some(vec!["student".to_string()]),
                        parameters: Some(DataSchema {
                            properties: indexmap::IndexMap::from([(
                                "course-id".to_string(),
                                PropertySchema {
                                    prop_type: Some("string".to_string()),
                                    format: Some("identifier".to_string()),
                                    ..Default::default()
                                },
                            )]),
                            required: vec!["course-id".to_string()],
                            ..Default::default()
                        }),
                        result: Some(PropertySchema {
                            prop_type: Some("object".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    EsdmDoc {
                        name: "list-course-rosters".to_string(),
                        kind: "query".to_string(),
                        read_model: Some("course-roster".to_string()),
                        actors: Some(vec!["student".to_string()]),
                        result: Some(PropertySchema {
                            prop_type: Some("array".to_string()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );

        let contexts = std::collections::HashMap::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = RoutingContext::default();

        generate_application(&out_dir, &bc_contexts, &contexts, &routing).unwrap();

        let bc_dir = resolve_bc_dir(&out_dir, "enrollment", &routing);
        let get_rs = std::fs::read_to_string(
            bc_dir.join("src/generated/application/query/get_course_roster.rs"),
        )
        .unwrap();
        let list_rs = std::fs::read_to_string(
            bc_dir.join("src/generated/application/query/list_course_rosters.rs"),
        )
        .unwrap();

        assert!(get_rs.contains("#[serde(rename = \"course-id\")]"));
        assert!(get_rs.contains("pub course_id: chromatofoor::Identifier"));
        assert!(!get_rs.contains("#[derive(Default)]"));
        assert!(get_rs.contains("self.course_id.to_string()"));
        assert!(get_rs.contains("records"));
        assert!(get_rs.contains(".get(&key)"));
        assert!(get_rs.contains("ProblemDetails::not_found"));
        assert!(syn::parse_file(&get_rs).is_ok());

        assert!(list_rs.contains("#[derive(Default)]"));
        assert!(list_rs.contains("type Output = Vec<CourseRosterRecord>;"));
        assert!(list_rs.contains("read_model.records.all().await"));
        assert!(syn::parse_file(&list_rs).is_ok());
    }
}
