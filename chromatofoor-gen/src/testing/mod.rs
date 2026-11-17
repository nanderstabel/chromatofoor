pub mod bounded_context;
pub mod process_manager;

pub use bounded_context::*;
pub use process_manager::*;

use crate::common::write_rust_file;
use anyhow::Result;
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

pub(crate) fn scenario_doc_comment(feature: &esdm::EsdmDoc, scenario: &esdm::Scenario) -> TokenStream {
    let mut lines = Vec::new();
    let feature_line = match feature.description.as_deref().map(str::trim) {
        Some(desc) if !desc.is_empty() => format!(" Feature: `{}` — {desc}", feature.name),
        _ => format!(" Feature: `{}`", feature.name),
    };
    lines.push(feature_line);
    lines.push(format!(" Scenario: `{}`", scenario.name));
    if let Some(desc) = scenario.description.as_deref().map(str::trim)
        && !desc.is_empty()
    {
        lines.push(String::new());
        for line in desc.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                lines.push(String::new());
            } else {
                lines.push(format!(" {trimmed}"));
            }
        }
    }
    quote! {
        #(#[doc = #lines])*
    }
}

pub(crate) fn sanitize_test_ident_segment(raw: &str) -> String {
    let snake = raw.to_case(Case::Snake);
    let mut out = String::with_capacity(snake.len());
    let mut prev_underscore = false;
    for ch in snake.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "scenario".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sync_generated_test_dir(crate_dir: &Path, modules: &[(String, TokenStream)]) -> Result<()> {
    let tests_dir = crate_dir.join("tests");
    if tests_dir.is_dir() {
        for entry in fs::read_dir(&tests_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("rs")
                && let Ok(content) = fs::read_to_string(&path)
                && content.lines().next() == Some("// ⚠️ DO NOT EDIT ⚠️")
            {
                fs::remove_file(&path)?;
            }
        }
    }

    let gen_dir = tests_dir.join("generated");
    match fs::remove_dir_all(&gen_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    if modules.is_empty() {
        if tests_dir.is_dir() && fs::read_dir(&tests_dir)?.next().is_none() {
            let _ = fs::remove_dir(&tests_dir);
        }
        return Ok(());
    }

    fs::create_dir_all(&gen_dir)?;

    let mut sorted_modules: Vec<&(String, TokenStream)> = modules.iter().collect();
    sorted_modules.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut mod_decls = Vec::with_capacity(sorted_modules.len());
    for (module_name, tokens) in sorted_modules {
        write_rust_file(&gen_dir.join(format!("{module_name}.rs")), tokens.clone())?;
        let mod_ident = format_ident!("{}", module_name);
        mod_decls.push(quote! { mod #mod_ident; });
    }

    let main_tokens = quote! {
        #![allow(
            unused_imports,
            clippy::doc_markdown,
            clippy::unwrap_used,
            clippy::expect_used,
            clippy::panic
        )]
        #(#mod_decls)*
    };
    write_rust_file(&gen_dir.join("main.rs"), main_tokens)?;

    Ok(())
}
