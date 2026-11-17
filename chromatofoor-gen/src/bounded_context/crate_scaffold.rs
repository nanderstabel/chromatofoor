use crate::common::{
    BoundedContextGenerator, MEMBER_LINTS_TOML, RoutingContext, ensure_manual_mod_file,
    get_relative_cargo_path, make_doc_comment, resolve_bc_dir, write_manual_rust_file,
    write_rust_file,
};
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn generate_bounded_context_scaffolds(
    out_dir: &Path,
    bc_aggregates: &HashMap<String, Vec<String>>,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    routing: &RoutingContext,
) -> Result<()> {
    let domain_crate_dir = out_dir.join("domain");
    let domain_pascal = routing.domain_ident(None);

    let mut sorted_bcs: Vec<&String> = bc_aggregates.keys().collect();
    sorted_bcs.sort();

    for bc in sorted_bcs {
        if bc == "global" {
            continue;
        }
        let _aggregates = bc_aggregates
            .get(bc)
            .ok_or_else(|| anyhow::anyhow!("BC aggregates not found: {bc}"))?;

        let bc_dir = resolve_bc_dir(out_dir, bc, routing);
        fs::create_dir_all(&bc_dir)?;

        let mut extra_dependencies = String::new();
        let mut has_external_system = false;
        let mut external_system_deps = std::collections::HashSet::new();

        if let Some(generator) = bc_contexts.get(bc) {
            let mut deps = std::collections::HashSet::new();
            for mapping in &generator.context_mappings {
                let is_dependent = match mapping.doc_type.as_deref() {
                    Some("customer-supplier") => mapping
                        .customer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("conformist") => mapping
                        .conformist
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("anti-corruption-layer") => mapping
                        .downstream
                        .as_ref()
                        .is_some_and(|d| d.bounded_context.as_deref() == Some(bc)),
                    Some("open-host-service") => mapping
                        .consumer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("published-language") => mapping
                        .consumer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    _ => false,
                };

                if is_dependent {
                    let target = match mapping.doc_type.as_deref() {
                        Some("customer-supplier") => mapping.supplier.as_ref(),
                        Some("conformist" | "anti-corruption-layer") => mapping.upstream.as_ref(),
                        Some("open-host-service") => mapping.host.as_ref(),
                        Some("published-language") => mapping.publisher.as_ref(),
                        _ => None,
                    };

                    if let Some(target) = target {
                        if let Some(target_bc) = &target.bounded_context {
                            deps.insert(target_bc.clone());
                        } else if let Some(ext_sys) = &target.external_system {
                            has_external_system = true;
                            external_system_deps.insert(ext_sys.clone());
                        }
                    }
                }
            }

            let mut sorted_deps: Vec<_> = deps.into_iter().collect();
            sorted_deps.sort();
            for dep in sorted_deps {
                let target_dir = resolve_bc_dir(out_dir, &dep, routing);
                let rel_path = get_relative_cargo_path(&bc_dir, &target_dir);
                extra_dependencies.push_str(&format!("    {dep} = {{ path = \"{rel_path}\" }}\n"));
            }
        }

        let domain_rel_path = get_relative_cargo_path(&bc_dir, &domain_crate_dir);

        let cargo_toml = format!(
            r#"[package]
    name = "{bc}"
    version = "0.1.0"
    edition = "2024"
    
    [dependencies]
    chromatofoor = {{ workspace = true }}
    domain = {{ path = "{domain_rel_path}" }}
    anyhow = {{ workspace = true }}
    async-trait = {{ workspace = true }}
    serde = {{ version = "1.0", features = ["derive"] }}
    thiserror = "2.0"
    uuid = {{ version = "1.11", features = ["serde", "v4"] }}
    chrono = {{ version = "0.4", features = ["serde"] }}
    serde_json = {{ workspace = true }}
    tracing = {{ workspace = true }}
{extra_dependencies}
    [dev-dependencies]
    tokio = {{ workspace = true }}
    rstest = "0.23"
    serde_json = {{ workspace = true }}
    tracing-test = {{ workspace = true }}
{MEMBER_LINTS_TOML}"#
        );
        fs::write(bc_dir.join("Cargo.toml"), cargo_toml)?;

        let src_dir = bc_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let mut lib_rs_content =
            "pub mod generated;\npub mod domain;\npub mod application;\n".to_string();
        if has_external_system {
            fs::create_dir_all(src_dir.join("infrastructure"))?;
            let mod_rs_path = src_dir.join("infrastructure").join("mod.rs");
            if !mod_rs_path.exists() {
                fs::write(
                    &mod_rs_path,
                    "// External system integrations (e.g. ACLs, HTTP clients) go here\n",
                )?;
            }

            let mut sorted_exts: Vec<_> = external_system_deps.into_iter().collect();
            sorted_exts.sort();
            let mut ext_mod_names = Vec::new();

            for ext in sorted_exts {
                let snake_ext = ext.to_case(Case::Snake);
                ext_mod_names.push(snake_ext.clone());

                let file_path = src_dir
                    .join("infrastructure")
                    .join(format!("{snake_ext}.rs"));
                if !file_path.exists() {
                    let ext_doc_tokens = if let Some(generator) = bc_contexts.get(bc)
                        && let Some(ext_doc) =
                            generator.external_systems.iter().find(|d| d.name == ext)
                    {
                        make_doc_comment(ext_doc.description.as_deref())
                    } else {
                        quote! {}
                    };

                    let trait_name = format_ident!("{}", ext.to_case(Case::Pascal));
                    let trait_code = quote! {
                        use async_trait::async_trait;

                        #ext_doc_tokens
                        #[async_trait]
                        pub trait #trait_name: Send + Sync {}
                    };
                    write_manual_rust_file(&file_path, trait_code)?;
                }
            }
            ensure_manual_mod_file(&mod_rs_path, &ext_mod_names)?;
            lib_rs_content.push_str("pub mod infrastructure;\n");
        }

        fs::write(src_dir.join("lib.rs"), lib_rs_content)?;

        let gen_dir = src_dir.join("generated");
        fs::create_dir_all(&gen_dir)?;
        write_rust_file(
            &gen_dir.join("mod.rs"),
            quote! {
                pub mod application;
                pub mod bootstrap;
                pub mod domain;
                pub mod prelude;
            },
        )?;

        let gen_domain_dir = gen_dir.join("domain");
        fs::create_dir_all(&gen_domain_dir)?;

        let mut gen_domain_mod_chunks = Vec::new();
        let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));
        gen_domain_mod_chunks.push(quote! {
            pub use ::domain::#domain_pascal;

            #[chromatofoor::bounded_context(scope = #domain_pascal)]
            pub struct #bc_pascal;
        });
        if let Some(aggregates) = bc_aggregates.get(bc) {
            let mut sorted_aggs = aggregates.clone();
            sorted_aggs.sort();
            for agg in &sorted_aggs {
                if agg == "global" {
                    continue;
                }
                let mod_name = format_ident!("{}", agg.to_case(Case::Snake));
                gen_domain_mod_chunks.push(quote! { pub mod #mod_name; });
            }
        }
        if let Some(ctx) = bc_contexts.get(bc) {
            if !ctx.entities.is_empty() {
                gen_domain_mod_chunks.push(quote! {
                    pub mod entities;
                    pub use entities::*;
                });
            }
            if !ctx.value_objects.is_empty() {
                gen_domain_mod_chunks.push(quote! {
                    pub mod value_objects;
                    pub use value_objects::*;
                });
            }
            if !ctx.domain_services.is_empty() {
                gen_domain_mod_chunks.push(quote! { pub mod services; });
            }
            if !ctx.actors.is_empty() {
                gen_domain_mod_chunks.push(quote! { pub mod actors; });
            }
        }
        let final_tokens = quote! { #(#gen_domain_mod_chunks)* };
        write_rust_file(&gen_domain_dir.join("mod.rs"), final_tokens)?;

        let domain_dir = src_dir.join("domain");
        if !domain_dir.exists() {
            fs::create_dir_all(&domain_dir)?;
        }

        let domain_mod_path = domain_dir.join("mod.rs");
        let mut domain_mod_names = Vec::new();
        if let Some(aggregates) = bc_aggregates.get(bc) {
            let mut sorted_aggs = aggregates.clone();
            sorted_aggs.sort();
            for agg in &sorted_aggs {
                if agg == "global" {
                    continue;
                }
                domain_mod_names.push(agg.to_case(Case::Snake));
            }
        }
        if let Some(ctx) = bc_contexts.get(bc)
            && !ctx.domain_services.is_empty()
        {
            domain_mod_names.push("services".to_string());
        }
        ensure_manual_mod_file(&domain_mod_path, &domain_mod_names)?;
    }

    Ok(())
}
