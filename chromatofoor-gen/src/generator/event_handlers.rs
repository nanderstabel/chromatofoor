use crate::model::EsdmDoc;
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub(crate) enum IntegrationHandlerKind {
    EventHandler,
    Policy,
}

pub fn generate_event_handlers(
    out_dir: &PathBuf,
    event_handlers: &[EsdmDoc],
    routing: &crate::generator::RoutingContext,
) -> Result<()> {
    generate_integration_handlers(
        out_dir,
        event_handlers,
        routing,
        IntegrationHandlerKind::EventHandler,
    )
}

pub(crate) fn generate_integration_handlers(
    out_dir: &PathBuf,
    docs: &[EsdmDoc],
    routing: &crate::generator::RoutingContext,
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

    // Determine dependencies
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

    let domain_rel_path =
        crate::generator::get_relative_cargo_path(&target_dir, &out_dir.join("domain"));

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
        let bc_dir = crate::generator::resolve_bc_dir(out_dir, bc, routing);
        let rel_path = pathdiff::diff_paths(&bc_dir, &target_dir).unwrap();
        let rel_path_str = rel_path.to_str().unwrap().replace('\\', "/");
        cargo_toml.push_str(&format!("{bc} = {{ path = \"{rel_path_str}\" }}\n"));
    }

    fs::write(target_dir.join("Cargo.toml"), cargo_toml)?;

    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    fs::write(
        src_dir.join("lib.rs"),
        "pub mod generated;\npub mod handlers;\n",
    )?;

    // Setup generated directories
    let gen_dir = src_dir.join("generated");
    fs::create_dir_all(&gen_dir)?;
    crate::generator::write_rust_file(&gen_dir.join("mod.rs"), quote! { pub mod handlers; })?;

    let gen_handlers_dir = gen_dir.join("handlers");
    fs::create_dir_all(&gen_handlers_dir)?;

    // Setup manual directories
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

        let mut handle_event_paths = Vec::new();
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
                let full_event_name =
                    crate::generator::format_scope_path(domain, &[bc, agg, &ev_kebab]);
                if seen_events.insert(full_event_name) {
                    let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
                    let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
                    let ev_mod = format_ident!("{}", ev.to_case(Case::Snake));
                    let ev_pascal = format_ident!("{}", ev.to_case(Case::Pascal));
                    handle_event_paths.push(quote! {
                        #bc_ident::generated::domain::#agg_snake::event::#ev_mod::#ev_pascal
                    });
                }
            }
        }

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
                let full_cmd_name =
                    crate::generator::format_scope_path(domain, &[bc, agg, &cmd_kebab]);
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

        let handler_doc = crate::generator::make_doc_comment(doc.description.as_deref());
        let dg_and_idem = crate::generator::format_delivery_and_idempotency_tokens(doc);

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

        crate::generator::write_rust_file(
            &gen_handlers_dir.join(format!("{handler_snake}.rs")),
            gen_content,
        )?;

        let manual_file = manual_handlers_dir.join(format!("{handler_snake}.rs"));
        if !manual_file.exists() {
            let manual_content = match kind {
                IntegrationHandlerKind::EventHandler => {
                    let deps_ident = format_ident!("{}Deps", handler_pascal);
                    let handle_impls = handle_event_paths.iter().map(|ev_path| {
                        quote! {
                            impl Handle<#ev_path> for #handler_pascal {
                                async fn handle(
                                    &self,
                                    _event: Envelope<#ev_path>,
                                ) -> Result<(), DomainError> {
                                    Ok(())
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
                    let react_impls = handle_event_paths.iter().map(|ev_path| {
                        quote! {
                            impl React<#ev_path> for #handler_pascal {
                                fn react(
                                    &self,
                                    _event: Envelope<#ev_path>,
                                ) -> Vec<#commands_enum_ident> {
                                    Vec::new()
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
            crate::generator::write_manual_rust_file(&manual_file, manual_content)?;
        }
    }

    let gen_mod_tokens = quote! { #(#gen_mod_chunks)* };
    crate::generator::write_rust_file(&gen_handlers_dir.join("mod.rs"), gen_mod_tokens)?;

    let manual_mod_file = manual_handlers_dir.join("mod.rs");
    crate::generator::ensure_manual_mod_file(&manual_mod_file, &manual_mod_names)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use insta::assert_snapshot;

    #[test]
    fn test_event_handlers_generation() {
        let eh = EsdmDoc {
            name: "TestEventHandler".to_string(),
            kind: "event-handler".to_string(),
            handles: Some(vec![EventRef {
                bounded_context: Some("sales".to_string()),
                aggregate: Some("order".to_string()),
                event: Some("OrderPlaced".to_string()),
            }]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = crate::generator::RoutingContext::default();
        generate_event_handlers(&out_dir, &[eh], &routing).unwrap();

        let eh_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/generated/handlers/test_event_handler.rs"),
        )
        .unwrap_or_default();

        assert_snapshot!("event_handler_generation", eh_rs);
    }

    #[test]
    fn test_event_handlers_disambiguates_colliding_events_in_trait_and_manual_file() {
        let eh = EsdmDoc {
            name: "AuditNotifier".to_string(),
            kind: "event-handler".to_string(),
            handles: Some(vec![
                EventRef {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    event: Some("Created".to_string()),
                },
                EventRef {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    event: Some("Created".to_string()),
                },
                EventRef {
                    bounded_context: Some("billing".to_string()),
                    aggregate: Some("invoice".to_string()),
                    event: Some("Created".to_string()),
                },
            ]),
            ..Default::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = crate::generator::RoutingContext::default();
        generate_event_handlers(&out_dir, &[eh], &routing).unwrap();

        let gen_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/generated/handlers/audit_notifier.rs"),
        )
        .unwrap();
        let manual_rs = std::fs::read_to_string(
            out_dir.join("integration/event-handlers/src/handlers/audit_notifier.rs"),
        )
        .unwrap();

        assert_eq!(
            gen_rs
                .matches("sales::generated::domain::order::event::created::Created")
                .count(),
            1
        );
        assert!(gen_rs.contains("billing::generated::domain::invoice::event::created::Created"));
        let normalized_manual: String = manual_rs.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized_manual.contains(
            "impl Handle<sales::generated::domain::order::event::created::Created> for AuditNotifier"
        ));
        assert!(normalized_manual.contains(
            "impl Handle<billing::generated::domain::invoice::event::created::Created> for AuditNotifier"
        ));
    }
}
