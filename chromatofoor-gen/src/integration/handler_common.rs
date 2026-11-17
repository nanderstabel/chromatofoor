use crate::common::{
    MEMBER_LINTS_TOML, RoutingContext, ensure_manual_mod_file,
    format_delivery_and_idempotency_tokens, format_scope_path, get_relative_cargo_path,
    make_doc_comment, resolve_bc_dir, write_manual_rust_file, write_rust_file,
};
use anyhow::Result;
use convert_case::{Case, Casing};
use esdm::EsdmDoc;
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub(crate) enum IntegrationHandlerKind {
    EventHandler,
    Policy,
}

pub(crate) fn generate_integration_handlers(
    out_dir: &PathBuf,
    docs: &[EsdmDoc],
    routing: &RoutingContext,
    kind: IntegrationHandlerKind,
) -> Result<()> {
    if docs.is_empty() {
        return Ok(());
    }

    let (pkg_name, extra_dep, include_emits_deps) = match kind {
        IntegrationHandlerKind::EventHandler => {
            ("event-handlers", "tracing = { workspace = true }", false)
        }
        IntegrationHandlerKind::Policy => (
            "policies",
            "uuid = { version = \"1.11\", features = [\"serde\", \"v4\"] }",
            true,
        ),
    };

    let target_dir = out_dir.join(format!("integration/{pkg_name}"));
    fs::create_dir_all(&target_dir)?;

    let mut dep_bcs = HashSet::new();
    for doc in docs {
        if let Some(handles) = &doc.handles {
            for h in handles {
                if let Some(bc) = &h.bounded_context {
                    dep_bcs.insert(bc.clone());
                }
            }
        }
        if include_emits_deps && let Some(emits) = &doc.emits {
            for e in emits {
                if let Some(bc) = &e.bounded_context {
                    dep_bcs.insert(bc.clone());
                }
            }
        }
    }

    let domain_rel_path = get_relative_cargo_path(&target_dir, &out_dir.join("domain"));

    let mut cargo_toml = format!(
        r#"[package]
name = "{pkg_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
chromatofoor = {{ workspace = true }}
domain = {{ path = "{domain_rel_path}" }}
anyhow = {{ workspace = true }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ workspace = true }}
{extra_dep}
"#
    );

    let mut sorted_dep_bcs: Vec<_> = dep_bcs.iter().collect();
    sorted_dep_bcs.sort();
    for bc in sorted_dep_bcs {
        let bc_dir = resolve_bc_dir(out_dir, bc, routing);
        let rel_path = pathdiff::diff_paths(&bc_dir, &target_dir).unwrap();
        let rel_path_str = rel_path.to_str().unwrap().replace('\\', "/");
        cargo_toml.push_str(&format!("{bc} = {{ path = \"{rel_path_str}\" }}\n"));
    }
    cargo_toml.push_str(MEMBER_LINTS_TOML);

    fs::write(target_dir.join("Cargo.toml"), cargo_toml)?;

    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    fs::write(
        src_dir.join("lib.rs"),
        "pub mod generated;\npub mod handlers;\n",
    )?;

    let gen_dir = src_dir.join("generated");
    fs::create_dir_all(&gen_dir)?;
    write_rust_file(&gen_dir.join("mod.rs"), quote! { pub mod handlers; })?;

    let gen_handlers_dir = gen_dir.join("handlers");
    fs::create_dir_all(&gen_handlers_dir)?;

    let manual_handlers_dir = src_dir.join("handlers");
    fs::create_dir_all(&manual_handlers_dir)?;

    let mut gen_mod_chunks = Vec::new();
    let mut manual_mod_names = Vec::new();

    for doc in docs {
        let handler_snake_str = doc.name.to_case(Case::Snake);
        let handler_snake = format_ident!("{}", handler_snake_str);
        let handler_pascal = format_ident!("{}", doc.name.to_case(Case::Pascal));
        let domain = routing.domain_or(doc.scope.as_ref().and_then(|s| s.domain.as_deref()));
        let domain_pascal =
            routing.domain_ident(doc.scope.as_ref().and_then(|s| s.domain.as_deref()));

        gen_mod_chunks.push(quote! { pub mod #handler_snake; });
        manual_mod_names.push(handler_snake_str);

        let mut handle_event_entries: Vec<(String, proc_macro2::TokenStream)> = Vec::new();
        let mut seen_events = std::collections::HashSet::new();

        if let Some(handles) = &doc.handles {
            for h in handles {
                let bc = h.bounded_context.as_deref().unwrap_or("global");
                let agg = h.aggregate.as_deref().unwrap_or("global");
                let ev = h.event.as_deref().unwrap_or("global");
                if bc == "global" || agg == "global" || ev == "global" {
                    continue;
                }
                let ev_kebab = ev.to_case(Case::Kebab);
                let full_event_name = format_scope_path(domain, &[bc, agg, &ev_kebab]);
                if seen_events.insert(full_event_name) {
                    let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
                    let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
                    let ev_mod = format_ident!("{}", ev.to_case(Case::Snake));
                    let ev_pascal_str = ev.to_case(Case::Pascal);
                    let ev_pascal = format_ident!("{}", ev_pascal_str);
                    handle_event_entries.push((
                        ev_pascal_str,
                        quote! {
                            #bc_ident::generated::domain::#agg_snake::event::#ev_mod::#ev_pascal
                        },
                    ));
                }
            }
        }
        let handle_event_paths: Vec<_> =
            handle_event_entries.iter().map(|(_, p)| p).collect();

        let mut emit_command_paths = Vec::new();
        let mut seen_commands = std::collections::HashSet::new();
        if matches!(kind, IntegrationHandlerKind::Policy)
            && let Some(emits) = &doc.emits
        {
            for e in emits {
                let bc = e.bounded_context.as_deref().unwrap_or("global");
                let agg = e.aggregate.as_deref().unwrap_or("global");
                let cmd = e.command.as_deref().unwrap_or("global");
                if bc == "global" || agg == "global" || cmd == "global" {
                    continue;
                }
                let cmd_kebab = cmd.to_case(Case::Kebab);
                let full_cmd_name = format_scope_path(domain, &[bc, agg, &cmd_kebab]);
                if seen_commands.insert(full_cmd_name) {
                    let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
                    let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
                    let cmd_mod = format_ident!("{}", cmd.to_case(Case::Snake));
                    let cmd_pascal = format_ident!("{}", cmd.to_case(Case::Pascal));
                    emit_command_paths.push(quote! {
                        #bc_ident::generated::domain::#agg_snake::command::#cmd_mod::#cmd_pascal
                    });
                }
            }
        }

        let handler_doc = make_doc_comment(doc.description.as_deref());
        let dg_and_idem = format_delivery_and_idempotency_tokens(doc);

        let gen_content = match kind {
            IntegrationHandlerKind::EventHandler => {
                let deps_ident = format_ident!("{}Deps", handler_pascal);
                quote! {
                    #handler_doc
                    #[chromatofoor::event_handler(
                        scope = ::domain::#domain_pascal,
                        handles = [#(#handle_event_paths),*],
                        #dg_and_idem
                    )]
                    #[derive(Clone, Default, Debug)]
                    pub struct #handler_pascal {
                        pub deps: crate::handlers::#handler_snake::#deps_ident,
                    }

                    impl std::ops::Deref for #handler_pascal {
                        type Target = crate::handlers::#handler_snake::#deps_ident;

                        fn deref(&self) -> &Self::Target {
                            &self.deps
                        }
                    }

                    impl std::ops::DerefMut for #handler_pascal {
                        fn deref_mut(&mut self) -> &mut Self::Target {
                            &mut self.deps
                        }
                    }
                }
            }
            IntegrationHandlerKind::Policy => {
                quote! {
                    #handler_doc
                    #[chromatofoor::policy(
                        scope = ::domain::#domain_pascal,
                        handles = [#(#handle_event_paths),*],
                        emits = [#(#emit_command_paths),*],
                        #dg_and_idem
                    )]
                    #[derive(Clone, Copy, Default, Debug)]
                    pub struct #handler_pascal;
                }
            }
        };

        write_rust_file(
            &gen_handlers_dir.join(format!("{handler_snake}.rs")),
            gen_content,
        )?;

        let manual_file = manual_handlers_dir.join(format!("{handler_snake}.rs"));
        if !manual_file.exists() {
            let manual_content = match kind {
                IntegrationHandlerKind::EventHandler => {
                    let deps_ident = format_ident!("{}Deps", handler_pascal);
                    let handle_impls =
                        handle_event_entries.iter().map(|(ev_pascal_str, ev_path)| {
                            let todo_msg = format!(
                                "implement Handle::{ev_pascal_str}::handle for {handler_pascal}"
                            );
                            quote! {
                                impl Handle<#ev_path> for #handler_pascal {
                                    async fn handle(
                                        &self,
                                        _event: Envelope<#ev_path>,
                                    ) -> Result<(), DomainError> {
                                        todo!(#todo_msg)
                                    }
                                }
                            }
                        });
                    quote! {
                        use crate::generated::handlers::#handler_snake::#handler_pascal;
                        use chromatofoor::{
                            architecturekit::{DomainError, Envelope},
                            integration::Handle,
                        };

                        #[derive(Clone, Default, Debug)]
                        pub struct #deps_ident {}

                        #(#handle_impls)*
                    }
                }
                IntegrationHandlerKind::Policy => {
                    let commands_enum_ident = format_ident!("{}Commands", handler_pascal);
                    let react_impls =
                        handle_event_entries.iter().map(|(ev_pascal_str, ev_path)| {
                            let todo_msg = format!(
                                "implement React::{ev_pascal_str}::react for {handler_pascal}"
                            );
                            quote! {
                                impl React<#ev_path> for #handler_pascal {
                                    fn react(
                                        &self,
                                        _event: Envelope<#ev_path>,
                                    ) -> Vec<#commands_enum_ident> {
                                        todo!(#todo_msg)
                                    }
                                }
                            }
                        });
                    quote! {
                        use crate::generated::handlers::#handler_snake::{
                            #commands_enum_ident, #handler_pascal,
                        };
                        use chromatofoor::{architecturekit::Envelope, integration::React};

                        #(#react_impls)*
                    }
                }
            };
            write_manual_rust_file(&manual_file, manual_content)?;
        }
    }

    let gen_mod_tokens = quote! { #(#gen_mod_chunks)* };
    write_rust_file(&gen_handlers_dir.join("mod.rs"), gen_mod_tokens)?;

    let manual_mod_file = manual_handlers_dir.join("mod.rs");
    ensure_manual_mod_file(&manual_mod_file, &manual_mod_names)?;

    Ok(())
}
