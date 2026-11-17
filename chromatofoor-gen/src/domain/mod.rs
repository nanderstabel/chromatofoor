use crate::common::{MEMBER_LINTS_TOML, RoutingContext, make_doc_comment, write_rust_file};
use anyhow::Result;
use convert_case::{Case, Casing};
use esdm::EsdmDoc;
use quote::{format_ident, quote};
use std::fs;
use std::path::Path;

pub fn generate_domain_crate(
    out_dir: &Path,
    all_esdm_docs: &[EsdmDoc],
    routing: &RoutingContext,
) -> Result<()> {
    let domain_crate_dir = out_dir.join("domain");
    let domain_crate_src = domain_crate_dir.join("src");
    fs::create_dir_all(&domain_crate_src)?;
    let domain_cargo_toml = format!(
        r#"[package]
name = "domain"
version = "0.1.0"
edition = "2024"

[dependencies]
chromatofoor = {{ workspace = true }}
{MEMBER_LINTS_TOML}"#
    );
    fs::write(domain_crate_dir.join("Cargo.toml"), domain_cargo_toml)?;

    let domain_pascal = routing.domain_ident(None);
    let domain_doc = all_esdm_docs
        .iter()
        .find(|doc| doc.kind == "domain")
        .and_then(|doc| doc.description.as_deref())
        .map_or_else(|| quote! {}, |d| make_doc_comment(Some(d)));

    let mut domain_lib_chunks = vec![quote! {
        #domain_doc
        #[chromatofoor::domain]
        pub struct #domain_pascal;
    }];

    let mut external_systems: Vec<&EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "external-system")
        .collect();
    external_systems.sort_by_key(|doc| &doc.name);

    for ext in external_systems {
        let ext_pascal = format_ident!("{}", ext.name.to_case(Case::Pascal));
        let ext_doc = make_doc_comment(ext.description.as_deref());
        let dir_ident = match ext.direction.as_deref().unwrap_or("outbound") {
            "inbound" => format_ident!("inbound"),
            "bidirectional" => format_ident!("bidirectional"),
            _ => format_ident!("outbound"),
        };
        let cat_attr = if let Some(cat) = &ext.category {
            quote! { , category = #cat }
        } else {
            quote! {}
        };
        domain_lib_chunks.push(quote! {
            #ext_doc
            #[chromatofoor::external_system(
                scope = #domain_pascal,
                direction = #dir_ident
                #cat_attr
            )]
            #[derive(Clone, Copy, Default, Debug)]
            pub struct #ext_pascal;
        });
    }

    write_rust_file(
        &domain_crate_src.join("lib.rs"),
        quote! { #(#domain_lib_chunks)* },
    )?;

    Ok(())
}
