use crate::common::{
    BoundedContextGenerator, RoutingContext, ensure_manual_mod_file, generate_struct_fields,
    make_doc_comment, make_doc_lines, resolve_bc_dir, write_manual_rust_file, write_rust_file,
};
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_domain_services(
    out_dir: &PathBuf,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    routing: &RoutingContext,
) -> Result<()> {
    let mut sorted_bcs: Vec<_> = bc_contexts.iter().collect();
    sorted_bcs.sort_by_key(|(bc, _)| *bc);

    for (bc, ctx) in sorted_bcs {
        if bc == "global" {
            continue;
        }

        let domain_services = &ctx.domain_services;
        if domain_services.is_empty() {
            continue;
        }

        let bc_dir = resolve_bc_dir(out_dir, bc, routing);
        let services_dir = bc_dir
            .join("src")
            .join("generated")
            .join("domain")
            .join("services");

        fs::create_dir_all(&services_dir)?;

        let manual_services_dir = bc_dir.join("src").join("domain").join("services");

        if !manual_services_dir.exists() {
            fs::create_dir_all(&manual_services_dir)?;
        }

        let mut mod_rs_lines = Vec::new();
        let mut manual_service_mods = Vec::new();

        let mut sorted_services: Vec<_> = domain_services.iter().collect();
        sorted_services.sort_by_key(|s| &s.name);

        for service in sorted_services {
            let service_snake_str = service.name.to_case(Case::Snake);
            let service_pascal_str = service.name.to_case(Case::Pascal);

            let _service_snake = format_ident!("{}", service_snake_str);
            let service_pascal = format_ident!("{}", service_pascal_str);
            let service_ident = format_ident!("{}Service", service_pascal_str);
            let service_snake_ident = format_ident!("{}", service_snake_str);

            mod_rs_lines.push(quote! {
                pub mod #service_snake_ident;
                pub use #service_snake_ident::*;
            });

            manual_service_mods.push(service_snake_str.clone());

            let mut methods = Vec::new();
            let mut manual_methods = Vec::new();
            let mut structs = Vec::new();
            let mut errors = Vec::new();

            if let Some(functions) = &service.functions {
                for func in functions {
                    let func_snake = format_ident!("{}", func.name.to_case(Case::Snake));
                    let func_pascal = format_ident!("{}", func.name.to_case(Case::Pascal));

                    let req_ident = format_ident!("{}Request", func_pascal);
                    let res_ident = format_ident!("{}Response", func_pascal);
                    let err_ident = format_ident!("{}Error", func_pascal);

                    let req_fields = generate_struct_fields(
                        &func.arguments,
                        &req_ident.to_string(),
                        &mut structs,
                    );
                    structs.push(quote! {
                        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, chromatofoor::validator::Validate)]
                        #[validate(crate = "chromatofoor::validator")]
                        pub struct #req_ident {
                            #(#req_fields,)*
                        }
                    });

                    let res_fields =
                        generate_struct_fields(&func.returns, &res_ident.to_string(), &mut structs);
                    structs.push(quote! {
                        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, chromatofoor::validator::Validate)]
                        #[validate(crate = "chromatofoor::validator")]
                        pub struct #res_ident {
                            #(#res_fields,)*
                        }
                    });

                    errors.push(quote! {
                        #[derive(Debug, thiserror::Error)]
                        pub enum #err_ident {
                            #[error("Internal error: {0}")]
                            Internal(String),
                            #[error("{0}")]
                            Validation(String),
                        }
                    });

                    let mut func_doc_lines = vec![match func.description.as_deref().map(str::trim) {
                        Some(d) if !d.is_empty() => format!("Function: `{}` — {d}", func.name),
                        _ => format!("Function: `{}`", func.name),
                    }];
                    if let Some(rules) = &func.rules
                        && !rules.is_empty()
                    {
                        func_doc_lines.push(String::new());
                        func_doc_lines.push("- Rules:".to_string());
                        for r in rules {
                            match r.rule.as_deref().map(str::trim) {
                                Some(text) if !text.is_empty() => {
                                    func_doc_lines.push(format!("  - `{}` — {text}", r.name));
                                }
                                _ => func_doc_lines.push(format!("  - `{}`", r.name)),
                            }
                        }
                    }
                    let func_doc = make_doc_lines(&func_doc_lines);

                    methods.push(quote! {
                        #func_doc
                        async fn #func_snake(&self, req: #req_ident) -> Result<#res_ident, #err_ident>;
                    });

                    let todo_msg =
                        format!("implement {service_ident}::{func_snake} for {service_pascal}");
                    manual_methods.push(quote! {
                        #func_doc
                        async fn #func_snake(&self, _req: crate::generated::domain::services::#service_snake_ident::#req_ident) -> Result<crate::generated::domain::services::#service_snake_ident::#res_ident, crate::generated::domain::services::#service_snake_ident::#err_ident> {
                            todo!(#todo_msg)
                        }
                    });
                }
            }

            let service_file_path = services_dir.join(format!("{service_snake_str}.rs"));
            let service_code = quote! {
                use async_trait::async_trait;
                #[allow(unused_imports)]
                use chromatofoor::validator::Validate;
                #[allow(unused_imports)]
                use serde::{Deserialize, Serialize};

                #(#structs)*

                #(#errors)*

                #[async_trait]
                pub trait #service_ident: Send + Sync {
                    #(#methods)*
                }
            };

            write_rust_file(&service_file_path, service_code)?;

            let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));
            let service_doc = make_doc_comment(service.description.as_deref());
            let manual_service_file_path =
                manual_services_dir.join(format!("{service_snake_str}.rs"));
            if !manual_service_file_path.exists() {
                let manual_service_code = quote! {
                    use async_trait::async_trait;

                    #service_doc
                    #[chromatofoor::domain_service(scope = crate::generated::domain::#bc_pascal)]
                    #[derive(Default, Clone)]
                    pub struct #service_pascal;

                    #[async_trait]
                    impl crate::generated::domain::services::#service_snake_ident::#service_ident for #service_pascal {
                        #(#manual_methods)*
                    }
                };
                write_manual_rust_file(&manual_service_file_path, manual_service_code)?;
            }
        }

        let mod_file_path = services_dir.join("mod.rs");
        let mod_code = quote! {
            #(#mod_rs_lines)*
        };
        write_rust_file(&mod_file_path, mod_code)?;

        let manual_mod_file_path = manual_services_dir.join("mod.rs");
        ensure_manual_mod_file(&manual_mod_file_path, &manual_service_mods)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::*;
    use insta::assert_snapshot;

    #[test]
    fn test_domain_service_generation() {
        let service = EsdmDoc {
            name: "CurrencyConversion".to_string(),
            kind: "domain-service".to_string(),
            functions: Some(vec![DomainFunction {
                name: "ConvertAmount".to_string(),
                arguments: DataSchema {
                    properties: indexmap::IndexMap::from([
                        (
                            "amount".to_string(),
                            PropertySchema {
                                prop_type: Some("integer".to_string()),
                                ..Default::default()
                            },
                        ),
                        (
                            "from_currency".to_string(),
                            PropertySchema {
                                prop_type: Some("string".to_string()),
                                ..Default::default()
                            },
                        ),
                        (
                            "to_currency".to_string(),
                            PropertySchema {
                                prop_type: Some("string".to_string()),
                                ..Default::default()
                            },
                        ),
                    ]),
                    required: vec![
                        "amount".to_string(),
                        "from_currency".to_string(),
                        "to_currency".to_string(),
                    ],
                    ..Default::default()
                },
                returns: DataSchema {
                    properties: indexmap::IndexMap::from([(
                        "converted_amount".to_string(),
                        PropertySchema {
                            prop_type: Some("integer".to_string()),
                            ..Default::default()
                        },
                    )]),
                    required: vec!["converted_amount".to_string()],
                    ..Default::default()
                },
                ..Default::default()
            }]),
            ..Default::default()
        };

        let mut bc_contexts = HashMap::new();
        let mut generator = BoundedContextGenerator::default();
        generator.domain_services.push(service);
        bc_contexts.insert("test_bc".to_string(), generator);

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &RoutingContext::default();
        generate_domain_services(&out_dir, &bc_contexts, routing).unwrap();

        let service_rs = std::fs::read_to_string({
            let test_bc_dir = resolve_bc_dir(&out_dir, "test_bc", routing);
            test_bc_dir.join("src/generated/domain/services/currency_conversion.rs")
        })
        .unwrap_or_default();
        let mod_rs = std::fs::read_to_string({
            let test_bc_dir = resolve_bc_dir(&out_dir, "test_bc", routing);
            test_bc_dir.join("src/generated/domain/services/mod.rs")
        })
        .unwrap_or_default();

        assert_snapshot!("domain_service_generation", service_rs);
        assert_snapshot!("domain_service_mod", mod_rs);
    }

    #[test]
    fn test_incremental_domain_service_mod_sync() {
        let make_service = |name: &str| EsdmDoc {
            name: name.to_string(),
            kind: "domain-service".to_string(),
            functions: Some(vec![]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = &RoutingContext::default();
        let test_bc_dir = resolve_bc_dir(&out_dir, "test_bc", routing);

        let mut bc_contexts = HashMap::new();
        let mut generator = BoundedContextGenerator::default();
        generator.domain_services.push(make_service("FirstService"));
        bc_contexts.insert("test_bc".to_string(), generator);
        generate_domain_services(&out_dir, &bc_contexts, routing).unwrap();

        let manual_mod_path = test_bc_dir.join("src/domain/services/mod.rs");
        std::fs::write(
            &manual_mod_path,
            "// mod second_service;\npub mod first_service;\npub mod second_service_helper;\n",
        )
        .unwrap();

        let mut bc_contexts2 = HashMap::new();
        let mut generator2 = BoundedContextGenerator::default();
        generator2
            .domain_services
            .push(make_service("FirstService"));
        generator2
            .domain_services
            .push(make_service("SecondService"));
        bc_contexts2.insert("test_bc".to_string(), generator2);
        generate_domain_services(&out_dir, &bc_contexts2, routing).unwrap();

        let updated_mod = std::fs::read_to_string(&manual_mod_path).unwrap();
        assert!(updated_mod.contains("// mod second_service;"));
        assert!(updated_mod.contains("pub mod second_service;\n"));
        assert_eq!(updated_mod.matches("pub mod first_service;").count(), 1);
    }
}
