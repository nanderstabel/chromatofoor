use esdm::EsdmDoc;
use pathdiff::diff_paths;
use proc_macro2::TokenStream;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    pub description: Option<String>,
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

pub fn make_doc_comment(desc: Option<&str>) -> TokenStream {
    match desc.map(str::trim) {
        Some(d) if !d.is_empty() => {
            let lines: Vec<String> = d
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        String::new()
                    } else {
                        format!(" {trimmed}")
                    }
                })
                .collect();
            quote::quote! { #(#[doc = #lines])* }
        }
        _ => quote::quote! {},
    }
}

pub fn make_doc_lines(lines: &[String]) -> TokenStream {
    if lines.is_empty() {
        return quote::quote! {};
    }
    let formatted: Vec<String> = lines
        .iter()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else {
                line.lines()
                    .map(|l| {
                        let trimmed = l.trim_end();
                        if trimmed.is_empty() {
                            String::new()
                        } else {
                            format!(" {trimmed}")
                        }
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect();
    quote::quote! { #(#[doc = #formatted])* }
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
        path_str.replace('\\', "/")
    } else {
        to_bc_dir.to_string_lossy().replace('\\', "/")
    }
}
