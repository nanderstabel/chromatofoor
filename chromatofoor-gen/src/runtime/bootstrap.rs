use crate::common::{BoundedContextGenerator, GeneratorContext, MEMBER_LINTS_TOML};
use anyhow::Result;
use convert_case::{Case, Casing};
use esdm::EsdmDoc;
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_bootstrap_crate(
    out_dir: &PathBuf,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    contexts: &HashMap<(String, String), GeneratorContext>,
    event_handlers: &[EsdmDoc],
    policies: &[EsdmDoc],
    process_managers: &[EsdmDoc],
    all_esdm_docs: &[EsdmDoc],
    routing: &crate::common::RoutingContext,
) -> Result<()> {
    let has_event_handlers = !event_handlers.is_empty();
    let has_policies = !policies.is_empty();
    let bootstrap_dir = out_dir.join("bootstrap");
    fs::create_dir_all(&bootstrap_dir)?;

    let mut all_bcs: std::collections::HashSet<String> = bc_contexts.keys().cloned().collect();
    for (bc, _) in contexts.keys() {
        all_bcs.insert(bc.clone());
    }
    let mut bcs: Vec<String> = all_bcs.into_iter().filter(|bc| bc != "global").collect();
    bcs.sort();

    // 1. Generate Cargo.toml
    let domain_rel_path =
        crate::common::get_relative_cargo_path(&bootstrap_dir, &out_dir.join("domain"));

    let mut cargo_toml = String::new();
    cargo_toml.push_str("[package]\nname = \"bootstrap\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n");
    cargo_toml.push_str("anyhow = { workspace = true }\n");
    cargo_toml.push_str("tokio = { workspace = true }\n");
    cargo_toml.push_str("tracing = { workspace = true }\n");
    cargo_toml.push_str("chromatofoor = { workspace = true }\n");
    cargo_toml.push_str(&format!("domain = {{ path = \"{domain_rel_path}\" }}\n"));

    for bc in &bcs {
        let rel_path = crate::common::get_relative_cargo_path(
            &bootstrap_dir,
            &crate::common::resolve_bc_dir(out_dir, bc, routing),
        );
        cargo_toml.push_str(&format!("{bc} = {{ path = \"{rel_path}\" }}\n"));
    }
    if has_event_handlers {
        cargo_toml.push_str("event-handlers = { path = \"../integration/event-handlers\" }\n");
    }
    if has_policies {
        cargo_toml.push_str("policies = { path = \"../integration/policies\" }\n");
    }
    let mut sorted_pms: Vec<&EsdmDoc> = process_managers.iter().collect();
    sorted_pms.sort_by_key(|pm| &pm.name);
    for pm in &sorted_pms {
        let pm_name = &pm.name;
        cargo_toml.push_str(&format!(
            "{pm_name} = {{ path = \"../integration/process-managers/{pm_name}\" }}\n"
        ));
    }
    cargo_toml.push_str(MEMBER_LINTS_TOML);

    fs::write(bootstrap_dir.join("Cargo.toml"), cargo_toml)?;

    // 2. Generate src/lib.rs
    let src_dir = bootstrap_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    let mut code_chunks = Vec::new();
    let domain_pascal = routing.domain_ident(None);

    code_chunks.push(quote! {
        #![allow(unused_imports, unused_variables)]
        pub use ::domain::#domain_pascal;
    });

    let mut subdomains: Vec<&EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "subdomain")
        .collect();
    subdomains.sort_by_key(|doc| &doc.name);

    for sub in subdomains {
        let sub_pascal = format_ident!("{}", sub.name.to_case(Case::Pascal));
        let sub_doc = crate::common::make_doc_comment(sub.description.as_deref());
        let sub_type_ident = match sub.doc_type.as_deref().unwrap_or("supporting") {
            "core" => format_ident!("core"),
            "generic" => format_ident!("generic"),
            _ => format_ident!("supporting"),
        };
        let mut sub_bc_paths = Vec::new();
        if let Some(covered_bcs) = &sub.bounded_contexts {
            let mut sorted_covered: Vec<&String> =
                covered_bcs.iter().filter(|bc| bcs.contains(*bc)).collect();
            sorted_covered.sort();
            for bc in sorted_covered {
                let bc_snake = format_ident!("{}", bc.to_case(Case::Snake));
                let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));
                sub_bc_paths.push(quote! { #bc_snake::generated::domain::#bc_pascal });
            }
        }
        if !sub_bc_paths.is_empty() {
            code_chunks.push(quote! {
                #sub_doc
                #[chromatofoor::subdomain(
                    scope = ::domain::#domain_pascal,
                    type = #sub_type_ident,
                    bounded_contexts = [#(#sub_bc_paths),*],
                )]
                pub struct #sub_pascal;
            });
        }
    }

    let mut consumer_registrations = Vec::new();

    let mut sorted_handlers: Vec<&EsdmDoc> = event_handlers.iter().collect();
    sorted_handlers.sort_by_key(|h| &h.name);
    for handler in sorted_handlers {
        let handler_snake = format_ident!("{}", handler.name.to_case(Case::Snake));
        let handler_pascal = format_ident!("{}", handler.name.to_case(Case::Pascal));

        consumer_registrations.push(quote! {
            builder = builder.register_consumer(
                event_handlers::generated::handlers::#handler_snake::#handler_pascal::default(),
            );
        });
    }

    let mut sorted_policies: Vec<&EsdmDoc> = policies.iter().collect();
    sorted_policies.sort_by_key(|p| &p.name);
    for policy in sorted_policies {
        let policy_snake = format_ident!("{}", policy.name.to_case(Case::Snake));
        let policy_pascal = format_ident!("{}", policy.name.to_case(Case::Pascal));

        consumer_registrations.push(quote! {
            builder = builder.register_consumer(
                policies::generated::handlers::#policy_snake::#policy_pascal,
            );
        });
    }

    for pm in &sorted_pms {
        let pm_snake = format_ident!("{}", pm.name.to_case(Case::Snake));
        let pm_pascal = format_ident!("{}", pm.name.to_case(Case::Pascal));

        consumer_registrations.push(quote! {
            builder = builder.register_consumer(#pm_snake::#pm_pascal::default());
        });
    }

    let mut bc_registrations = Vec::new();
    for bc in &bcs {
        let bc_snake = format_ident!("{}", bc.to_case(Case::Snake));

        bc_registrations.push(quote! {
            builder = #bc_snake::generated::bootstrap::register(builder);
        });
    }

    let mut context_mappings: Vec<&EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "context-mapping")
        .collect();
    context_mappings.sort_by_key(|doc| &doc.name);

    let resolve_endpoint = |end: &esdm::MappingEnd| -> Option<proc_macro2::TokenStream> {
        if let Some(bc) = &end.bounded_context {
            if !bcs.contains(bc) {
                return None;
            }
            let bc_snake = format_ident!("{}", bc.to_case(Case::Snake));
            let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));
            Some(quote! { #bc_snake::generated::domain::#bc_pascal })
        } else if let Some(ext) = &end.external_system {
            let ext_pascal = format_ident!("{}", ext.to_case(Case::Pascal));
            Some(quote! { ::domain::#ext_pascal })
        } else {
            None
        }
    };

    for cm in context_mappings {
        let cm_pascal = format_ident!("{}", cm.name.to_case(Case::Pascal));
        let cm_doc = crate::common::make_doc_comment(cm.description.as_deref());
        let Some(doc_type) = cm.doc_type.as_deref() else {
            continue;
        };
        let type_ident = format_ident!("{}", doc_type.to_case(Case::Snake));

        let mapping_tokens = match doc_type {
            "customer-supplier" => {
                if let (Some(c), Some(s)) = (
                    cm.customer.as_ref().and_then(resolve_endpoint),
                    cm.supplier.as_ref().and_then(resolve_endpoint),
                ) {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            customer = #c,
                            supplier = #s,
                        )]
                    })
                } else {
                    None
                }
            }
            "conformist" => {
                if let (Some(c), Some(u)) = (
                    cm.conformist.as_ref().and_then(resolve_endpoint),
                    cm.upstream.as_ref().and_then(resolve_endpoint),
                ) {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            conformist = #c,
                            upstream = #u,
                        )]
                    })
                } else {
                    None
                }
            }
            "anti-corruption-layer" => {
                if let (Some(d), Some(u)) = (
                    cm.downstream.as_ref().and_then(resolve_endpoint),
                    cm.upstream.as_ref().and_then(resolve_endpoint),
                ) {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            downstream = #d,
                            upstream = #u,
                        )]
                    })
                } else {
                    None
                }
            }
            "open-host-service" => {
                if let (Some(h), Some(c)) = (
                    cm.host.as_ref().and_then(resolve_endpoint),
                    cm.consumer.as_ref().and_then(resolve_endpoint),
                ) {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            host = #h,
                            consumer = #c,
                        )]
                    })
                } else {
                    None
                }
            }
            "published-language" => {
                if let (Some(p), Some(c)) = (
                    cm.publisher.as_ref().and_then(resolve_endpoint),
                    cm.consumer.as_ref().and_then(resolve_endpoint),
                ) {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            publisher = #p,
                            consumer = #c,
                        )]
                    })
                } else {
                    None
                }
            }
            "shared-kernel" | "partnership" | "separate-ways" => {
                if let Some(parts) = &cm.participants
                    && parts.len() == 2
                    && let (Some(ep1), Some(ep2)) =
                        (resolve_endpoint(&parts[0]), resolve_endpoint(&parts[1]))
                {
                    Some(quote! {
                        #[chromatofoor::context_mapping(
                            type = #type_ident,
                            participants = [#ep1, #ep2],
                        )]
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(attr_tokens) = mapping_tokens {
            code_chunks.push(quote! {
                #cm_doc
                #attr_tokens
                pub struct #cm_pascal;
            });
        }
    }

    code_chunks.push(quote! {
        pub struct Domain;

        impl Domain {
            pub async fn build_with_store(
                store: std::sync::Arc<chromatofoor::architecturekit::Store>,
            ) -> Result<chromatofoor::domain::DomainRuntime, Box<dyn std::error::Error + Send + Sync>> {
                let mut builder = chromatofoor::domain::DomainBuilder::<::domain::#domain_pascal>::new(store);
                #(#bc_registrations)*
                #(#consumer_registrations)*

                builder.build().await
            }

            pub async fn build() -> Result<chromatofoor::domain::DomainRuntime, Box<dyn std::error::Error + Send + Sync>> {
                let store = chromatofoor::architecturekit::Store::builder(
                    chromatofoor::architecturekit::InMemoryEventStore::new(),
                    "https://chromatofoor.local",
                )
                .build();
                Self::build_with_store(std::sync::Arc::new(store)).await
            }
        }
    });

    let final_tokens = quote! { #(#code_chunks)* };
    crate::common::write_rust_file(&src_dir.join("lib.rs"), final_tokens)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{BoundedContextGenerator, GeneratorContext};
    use esdm::*;
    use insta::assert_snapshot;

    #[test]
    fn test_bootstrap_generation() {
        let aggregate = EsdmDoc {
            name: "TestAggregate".to_string(),
            kind: "aggregate".to_string(),
            ..Default::default()
        };

        let command = EsdmDoc {
            name: "CreateTest".to_string(),
            kind: "command".to_string(),
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
        };

        let mut contexts = std::collections::HashMap::new();
        contexts.insert(
            ("TestContext".to_string(), "TestAggregate".to_string()),
            GeneratorContext {
                commands: vec![command],
                events: vec![],
                aggregate: Some(aggregate),
                boundary: None,
                lifecycle: None,
                features: vec![],
                entities: vec![],
                value_objects: vec![],
            },
        );

        let mut bc_contexts = std::collections::HashMap::new();
        bc_contexts.insert(
            "TestContext".to_string(),
            BoundedContextGenerator {
                description: None,
                features: vec![],
                entities: vec![],
                value_objects: vec![],
                read_models: vec![],
                queries: vec![],
                domain_services: vec![],
                actors: vec![],
                context_mappings: vec![],
                external_systems: vec![],
            },
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &crate::common::RoutingContext::default();
        generate_bootstrap_crate(
            &out_dir,
            &bc_contexts,
            &contexts,
            &[],
            &[],
            &[],
            &[],
            routing,
        )
        .unwrap();

        let lib_rs =
            std::fs::read_to_string(out_dir.join("bootstrap/src/lib.rs")).unwrap_or_default();
        let cargo_toml =
            std::fs::read_to_string(out_dir.join("bootstrap/Cargo.toml")).unwrap_or_default();
        assert!(cargo_toml.contains("[lints]\nworkspace = true"));

        assert_snapshot!("bootstrap_generation", lib_rs);
    }
}
