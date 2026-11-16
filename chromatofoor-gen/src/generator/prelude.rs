use crate::generator::{BoundedContextGenerator, GeneratorContext};
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_preludes(
    out_dir: &PathBuf,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    contexts: &HashMap<(String, String), GeneratorContext>,
    routing: &crate::generator::RoutingContext,
) -> Result<()> {
    let mut bc_preludes: HashMap<String, Vec<proc_macro2::TokenStream>> = HashMap::new();

    let mut sorted_contexts: Vec<_> = contexts.keys().collect();
    sorted_contexts.sort();

    for key in sorted_contexts {
        let (bc, agg_name) = key;
        let _ctx = contexts
            .get(key)
            .ok_or_else(|| anyhow::anyhow!("Context not found"))?;
        if bc == "global" || agg_name == "global" {
            continue;
        }
        let agg_snake = format_ident!("{}", agg_name.to_case(Case::Snake));
        let agg_pascal = format_ident!("{}", agg_name.to_case(Case::Pascal));
        let command_enum = format_ident!("{}Command", agg_pascal);
        let error_name = format_ident!("{}Error", agg_pascal);

        let mut cmd_exports = Vec::new();
        for cmd in &_ctx.commands {
            let cmd_snake = format_ident!("{}", cmd.name.to_case(Case::Snake));
            let cmd_pascal = format_ident!("{}", cmd.name.to_case(Case::Pascal));
            cmd_exports.push(quote! {
                pub use crate::generated::domain::#agg_snake::command::#cmd_snake::#cmd_pascal;
            });
        }

        let prelude_list = bc_preludes.entry(bc.clone()).or_default();
        prelude_list.push(quote! {
            pub use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
            pub use crate::generated::domain::#agg_snake::error::#error_name;
            pub use crate::generated::domain::#agg_snake::command::#command_enum;
            #(#cmd_exports)*
        });
    }

    let mut sorted_bc_contexts: Vec<_> = bc_contexts.keys().collect();
    sorted_bc_contexts.sort();

    for bc in sorted_bc_contexts {
        let bc_ctx = bc_contexts
            .get(bc)
            .ok_or_else(|| anyhow::anyhow!("BC context not found: {bc}"))?;
        if bc == "global" {
            continue;
        }
        let prelude_list = bc_preludes.entry(bc.clone()).or_default();
        if !bc_ctx.entities.is_empty() {
            prelude_list.push(quote! {
                pub use crate::generated::domain::entities::*;
            });
        }
        if !bc_ctx.value_objects.is_empty() {
            prelude_list.push(quote! {
                pub use crate::generated::domain::value_objects::*;
            });
        }
        for rm in &bc_ctx.read_models {
            let rm_pascal = format_ident!("{}", rm.name.to_case(Case::Pascal));
            let rm_record_pascal = format_ident!("{}Record", rm_pascal);
            let mod_name = format_ident!("{}", rm.name.to_case(Case::Snake));

            prelude_list.push(quote! {
                pub use crate::generated::application::read_models::#mod_name::{#rm_pascal, #rm_record_pascal};
            });
        }

        for q in &bc_ctx.queries {
            let q_pascal = format_ident!("{}", q.name.to_case(Case::Pascal));
            let q_snake = format_ident!("{}", q.name.to_case(Case::Snake));
            prelude_list.push(quote! {
                pub use crate::generated::application::query::#q_snake::#q_pascal;
            });
        }
    }

    let mut sorted_bc_preludes: Vec<_> = bc_preludes.keys().collect();
    sorted_bc_preludes.sort();

    for bc in sorted_bc_preludes {
        let prelude_chunks = bc_preludes
            .get(bc)
            .ok_or_else(|| anyhow::anyhow!("BC prelude not found: {bc}"))?;
        let bc_dir = crate::generator::resolve_bc_dir(out_dir, bc, routing);
        let prelude_dir = bc_dir.join("src").join("generated");
        fs::create_dir_all(&prelude_dir)?;

        let tokens = quote! {
            #(#prelude_chunks)*
        };

        crate::generator::write_rust_file(&prelude_dir.join("prelude.rs"), tokens)?;

        // --- Generate src/generated/bootstrap.rs for this bounded context ---
        let mut register_cmds = Vec::new();
        let mut sorted_contexts: Vec<_> = contexts.iter().collect();
        sorted_contexts.sort_by_key(|((bc_name, agg), _)| (bc_name.clone(), agg.clone()));
        for ((ctx_bc, agg_name), ctx) in sorted_contexts {
            if ctx_bc != bc || agg_name == "global" {
                continue;
            }

            let agg_pascal = format_ident!("{}", agg_name.to_case(Case::Pascal));
            for cmd in &ctx.commands {
                let cmd_name = &cmd.name;
                let cmd_pascal = format_ident!("{}", cmd_name.to_case(Case::Pascal));

                register_cmds.push(quote! {
                    builder = builder.register_command::<#cmd_pascal, #agg_pascal>();
                });
            }
        }

        let mut register_read_models = Vec::new();
        if let Some(bc_ctx) = bc_contexts.get(bc) {
            for rm in &bc_ctx.read_models {
                let rm_pascal = format_ident!("{}", rm.name.to_case(Case::Pascal));
                let rm_var = format_ident!("{}_rm", rm.name.to_case(Case::Snake));

                let mut query_regs = Vec::new();
                for q in &bc_ctx.queries {
                    let q_rm = q.read_model.as_deref().unwrap_or(&q.name);
                    if q_rm == rm.name || q.name == rm.name {
                        let query_pascal = format_ident!("{}", q.name.to_case(Case::Pascal));
                        query_regs.push(quote! {
                            builder = builder.register_query::<#query_pascal, #rm_pascal>(#rm_var.clone());
                        });
                    }
                }

                register_read_models.push(quote! {
                    let #rm_var = Arc::new(#rm_pascal::new());
                    builder = builder.register_read_model::<#rm_pascal>(#rm_var.clone());
                    #(#query_regs)*
                });
            }
        }

        let bootstrap_tokens = quote! {
            use crate::generated::prelude::*;
            use chromatofoor::domain::{Domain, DomainBuilder};
            #[allow(unused_imports)]
            use std::sync::Arc;

            pub fn register<D: Domain>(mut builder: DomainBuilder<D>) -> DomainBuilder<D> {
                #(#register_cmds)*
                #(#register_read_models)*
                builder
            }
        };

        crate::generator::write_rust_file(&prelude_dir.join("bootstrap.rs"), bootstrap_tokens)?;
    }

    Ok(())
}
