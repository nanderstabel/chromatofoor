pub mod application;
pub mod bootstrap;
pub mod domain;
pub mod domain_service;
pub mod event_handlers;
pub mod graph;
pub mod policies;
pub mod prelude;
pub mod process_manager;
pub mod process_manager_tests;
pub mod tests;
#[cfg(feature = "typescript")]
pub mod typescript;
pub mod workspace;

use crate::model::EsdmDoc;

#[derive(Default)]
pub struct GeneratorContext {
    pub commands: Vec<EsdmDoc>,
    pub events: Vec<EsdmDoc>,
    pub aggregate: Option<EsdmDoc>,
    pub boundary: Option<EsdmDoc>,
    pub lifecycle: Option<EsdmDoc>,
    pub features: Vec<EsdmDoc>,
    pub entities: Vec<EsdmDoc>,
    pub value_objects: Vec<EsdmDoc>,
}

#[derive(Default)]
pub struct BoundedContextGenerator {
    pub read_models: Vec<EsdmDoc>,
    pub queries: Vec<EsdmDoc>,
    pub features: Vec<EsdmDoc>,
    pub entities: Vec<EsdmDoc>,
    pub value_objects: Vec<EsdmDoc>,
    pub domain_services: Vec<EsdmDoc>,
    pub actors: Vec<EsdmDoc>,
    pub context_mappings: Vec<EsdmDoc>,
    pub external_systems: Vec<EsdmDoc>,
}

use anyhow::{Context, Result};
use pathdiff::diff_paths;
use proc_macro2::TokenStream;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn make_doc_comment(desc: Option<&str>) -> TokenStream {
    match desc {
        Some(d) if !d.trim().is_empty() => {
            let d_clean = format!(" {}", d.trim());
            quote::quote! { #[doc = #d_clean] }
        }
        _ => quote::quote! {},
    }
}

#[derive(Default, Clone)]
pub struct RoutingContext {
    pub domain_name: String,
    pub bc_to_subdomain: HashMap<String, String>,
    pub bc_to_tier: HashMap<String, String>,
}

impl RoutingContext {
    pub fn domain_or<'a>(&'a self, doc_domain: Option<&'a str>) -> Option<&'a str> {
        if self.domain_name.is_empty() {
            doc_domain
        } else {
            Some(&self.domain_name)
        }
    }

    pub fn domain_ident(&self, doc_domain: Option<&str>) -> syn::Ident {
        use convert_case::{Case, Casing};
        let domain_str = self.domain_or(doc_domain).unwrap_or("global-domain");
        quote::format_ident!("{}", domain_str.to_case(Case::Pascal))
    }
}

pub fn format_delivery_and_idempotency_tokens(doc: &EsdmDoc) -> TokenStream {
    let dg = doc.delivery_guarantee.as_deref();
    let idem = doc.idempotency.as_ref();

    let owner_ident_opt = idem.map(|i| match i.owner.as_str() {
        "self" | "self-managed" | "self_managed" => quote::format_ident!("self_managed"),
        "downstream" => quote::format_ident!("downstream"),
        "infrastructure" => quote::format_ident!("infrastructure"),
        "none" => quote::format_ident!("none"),
        "not-required" | "not_required" => quote::format_ident!("not_required"),
        _ => quote::format_ident!("self_managed"),
    });

    let idem_tokens = if let Some(owner_ident) = owner_ident_opt {
        if let Some(strat) = idem.and_then(|i| i.strategy.as_deref()) {
            quote::quote! { idempotency(owner = #owner_ident, strategy = #strat), }
        } else {
            quote::quote! { idempotency = #owner_ident, }
        }
    } else if dg == Some("at-least-once") {
        quote::quote! { idempotency = self_managed, }
    } else {
        quote::quote! {}
    };

    let dg_tokens = match dg {
        Some("at-least-once") => quote::quote! { delivery_guarantee = at_least_once, },
        Some("at-most-once") => quote::quote! { delivery_guarantee = at_most_once, },
        _ => quote::quote! {},
    };

    quote::quote! {
        #dg_tokens
        #idem_tokens
    }
}

pub fn format_scope_path(domain: Option<&str>, segments: &[&str]) -> String {
    use convert_case::{Case, Casing};
    let mut out = String::new();
    if let Some(d) = domain.filter(|d| !d.is_empty() && *d != "global") {
        out.push('/');
        out.push_str(&d.to_case(Case::Kebab));
    }
    for seg in segments {
        if !seg.is_empty() && *seg != "global" {
            out.push('/');
            out.push_str(&seg.to_case(Case::Kebab));
        }
    }
    out
}

pub fn resolve_bc_dir(out_dir: &Path, bc_name: &str, routing: &RoutingContext) -> PathBuf {
    if let Some(tier) = routing.bc_to_tier.get(bc_name)
        && tier == "shared-kernel"
    {
        return out_dir.join("shared-kernels").join(bc_name);
    }
    if let Some(subdomain) = routing.bc_to_subdomain.get(bc_name) {
        return out_dir
            .join("bounded-contexts")
            .join(subdomain)
            .join(bc_name);
    }
    out_dir.join("bounded-contexts").join(bc_name)
}

pub fn get_relative_cargo_path(from_bc_dir: &Path, to_bc_dir: &Path) -> String {
    if let Some(diff) = diff_paths(to_bc_dir, from_bc_dir) {
        let path_str = diff.to_string_lossy().to_string();
        // Replace windows backslashes with forward slashes for Cargo.toml
        path_str.replace('\\', "/")
    } else {
        // Fallback if pathdiff fails (shouldn't happen for absolute paths or same base)
        to_bc_dir.to_string_lossy().replace('\\', "/")
    }
}

pub fn write_generated_rust_file(path: &Path, tokens: TokenStream) -> Result<()> {
    let raw_code = tokens.to_string();
    let syntax_tree = syn::parse_file(&raw_code).with_context(|| {
        format!("Failed to parse generated code for {path:?}\nCode:\n{raw_code}")
    })?;
    let mut formatted_code = prettyplease::unparse(&syntax_tree);
    formatted_code = format!(
        "// ⚠️ DO NOT EDIT ⚠️\n// This file was generated by chromatofoor-gen.\n// Any manual changes to this file will be overwritten on the next generation run.\n\n{formatted_code}"
    );
    fs::write(path, formatted_code)?;
    Ok(())
}

pub fn write_manual_rust_file(path: &Path, tokens: TokenStream) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let raw_code = tokens.to_string();
    let syntax_tree = syn::parse_file(&raw_code).with_context(|| {
        format!("Failed to parse manual scaffold code for {path:?}\nCode:\n{raw_code}")
    })?;
    let formatted_code = prettyplease::unparse(&syntax_tree);
    fs::write(path, formatted_code)?;
    Ok(())
}

fn mod_ident(name: &str) -> syn::Ident {
    let clean = name.strip_prefix("r#").unwrap_or(name);
    syn::parse_str::<syn::Ident>(clean)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{clean}")))
        .unwrap_or_else(|_| quote::format_ident!("{}_mod", clean))
}

pub fn ensure_manual_mod_file(path: &Path, mod_names: &[String]) -> Result<()> {
    if !path.exists() {
        let mods: Vec<_> = mod_names
            .iter()
            .map(|m| {
                let ident = mod_ident(m);
                quote::quote! { pub mod #ident; }
            })
            .collect();
        return write_manual_rust_file(path, quote::quote! { #(#mods)* });
    }

    let mut content = fs::read_to_string(path)?;
    let existing_mods: std::collections::HashSet<String> = match syn::parse_file(&content) {
        Ok(syntax_tree) => syntax_tree
            .items
            .into_iter()
            .filter_map(|item| match item {
                syn::Item::Mod(m) => Some(syn::ext::IdentExt::unraw(&m.ident).to_string()),
                _ => None,
            })
            .collect(),
        Err(_) => std::collections::HashSet::new(),
    };

    let mut changed = false;
    for mod_name in mod_names {
        let ident = mod_ident(mod_name);
        let clean_name = syn::ext::IdentExt::unraw(&ident).to_string();
        if !existing_mods.contains(&clean_name) {
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&format!("pub mod {ident};\n"));
            changed = true;
        }
    }
    if changed {
        fs::write(path, content)?;
    }
    Ok(())
}

pub fn write_rust_file(path: &Path, tokens: TokenStream) -> Result<()> {
    write_generated_rust_file(path, tokens)
}

#[cfg(test)]
mod mod_file_tests {
    use super::*;

    #[test]
    fn ensure_manual_mod_file_handles_comments_inline_mods_and_keywords() {
        let dir = tempfile::tempdir().unwrap();
        let mod_path = dir.path().join("mod.rs");

        // 1. Fresh creation with normal and keyword modules
        ensure_manual_mod_file(&mod_path, &["alpha".to_string(), "type".to_string()]).unwrap();
        let created = fs::read_to_string(&mod_path).unwrap();
        assert!(created.contains("pub mod alpha;"));
        assert!(created.contains("pub mod r#type;"));
        assert!(syn::parse_file(&created).is_ok());

        // 2. Existing file with commented-out `pub mod beta;`, inline `mod gamma {}`, and `pub(crate) mod delta;`
        let custom = "// pub mod beta;\n/* pub mod epsilon; */\nmod gamma {}\npub(crate) mod delta;\npub mod r#type;\n";
        fs::write(&mod_path, custom).unwrap();

        ensure_manual_mod_file(
            &mod_path,
            &[
                "beta".to_string(),
                "gamma".to_string(),
                "delta".to_string(),
                "epsilon".to_string(),
                "type".to_string(),
                "match".to_string(),
            ],
        )
        .unwrap();

        let updated = fs::read_to_string(&mod_path).unwrap();
        let parsed = syn::parse_file(&updated).unwrap();
        let mod_idents: Vec<String> = parsed
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(m) => Some(syn::ext::IdentExt::unraw(&m.ident).to_string()),
                _ => None,
            })
            .collect();
        // Should have gamma, delta, type (once each), plus newly appended beta, epsilon, match
        assert_eq!(
            mod_idents,
            vec!["gamma", "delta", "type", "beta", "epsilon", "match"]
        );
        assert!(updated.contains("pub mod r#match;"));
    }
}
