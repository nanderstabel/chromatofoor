use crate::common::{
    MEMBER_LINTS_TOML, RoutingContext, ensure_manual_mod_file,
    format_delivery_and_idempotency_tokens, format_scope_path, get_relative_cargo_path,
    make_doc_comment, make_doc_lines, resolve_bc_dir, write_manual_rust_file, write_rust_file,
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

        let mut raw_handles: Vec<(String, String, String, String)> = Vec::new();
        let mut seen_events = HashSet::new();
        let mut ev_pascal_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

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
                    let ev_pascal_str = ev.to_case(Case::Pascal);
                    *ev_pascal_counts.entry(ev_pascal_str.clone()).or_insert(0) += 1;
                    raw_handles.push((bc.to_string(), agg.to_string(), ev_kebab, ev_pascal_str));
                }
            }
        }

        let handler_pascal_str = handler_pascal.to_string();
        let domain_pascal_str = domain_pascal.to_string();
        let mut handle_imports = std::collections::BTreeSet::new();
        let mut handle_event_entries: Vec<(String, String, String, String, syn::Ident)> =
            Vec::new();
        for (bc, agg, ev_kebab, ev_pascal_str) in raw_handles {
            let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
            let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
            let ev_pascal = format_ident!("{}", ev_pascal_str);
            let collides = ev_pascal_counts.get(&ev_pascal_str).copied().unwrap_or(0) > 1
                || ev_pascal_str == handler_pascal_str
                || ev_pascal_str == domain_pascal_str;
            let local_ident = if collides {
                let alias = format_ident!(
                    "{}{}{}",
                    bc.to_case(Case::Pascal),
                    agg.to_case(Case::Pascal),
                    ev_pascal_str
                );
                handle_imports.insert(
                    quote! {
                        pub use #bc_ident::generated::domain::#agg_snake::event::#ev_pascal as #alias;
                    }
                    .to_string(),
                );
                alias
            } else {
                handle_imports.insert(
                    quote! {
                        pub use #bc_ident::generated::domain::#agg_snake::event::#ev_pascal;
                    }
                    .to_string(),
                );
                ev_pascal
            };
            handle_event_entries.push((bc, agg, ev_kebab, ev_pascal_str, local_ident));
        }

        let handle_event_idents: Vec<_> = handle_event_entries
            .iter()
            .map(|(_, _, _, _, ident)| ident)
            .collect();
        let parsed_handle_imports: Vec<proc_macro2::TokenStream> = handle_imports
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut raw_emits: Vec<(String, String, String)> = Vec::new();
        let mut seen_commands = HashSet::new();
        let mut cmd_pascal_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
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
                    let cmd_pascal_str = cmd.to_case(Case::Pascal);
                    *cmd_pascal_counts.entry(cmd_pascal_str.clone()).or_insert(0) += 1;
                    raw_emits.push((bc.to_string(), agg.to_string(), cmd_pascal_str));
                }
            }
        }

        let mut emit_imports = std::collections::BTreeSet::new();
        let mut emit_command_idents = Vec::new();
        for (bc, agg, cmd_pascal_str) in raw_emits {
            let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
            let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
            let cmd_pascal = format_ident!("{}", cmd_pascal_str);
            let collides = cmd_pascal_counts.get(&cmd_pascal_str).copied().unwrap_or(0) > 1
                || cmd_pascal_str == handler_pascal_str
                || cmd_pascal_str == domain_pascal_str
                || handle_event_idents.iter().any(|id| **id == cmd_pascal);
            let local_ident = if collides {
                let alias = format_ident!(
                    "{}{}{}",
                    bc.to_case(Case::Pascal),
                    agg.to_case(Case::Pascal),
                    cmd_pascal_str
                );
                emit_imports.insert(
                    quote! {
                        pub use #bc_ident::generated::domain::#agg_snake::command::#cmd_pascal as #alias;
                    }
                    .to_string(),
                );
                alias
            } else {
                emit_imports.insert(
                    quote! {
                        pub use #bc_ident::generated::domain::#agg_snake::command::#cmd_pascal;
                    }
                    .to_string(),
                );
                cmd_pascal
            };
            emit_command_idents.push(local_ident);
        }
        let parsed_emit_imports: Vec<proc_macro2::TokenStream> =
            emit_imports.iter().filter_map(|s| s.parse().ok()).collect();

        let (domain_import, domain_scope_ref) = if domain_pascal_str == handler_pascal_str {
            let alias = format_ident!("{}Domain", domain_pascal);
            (
                quote! { use ::domain::#domain_pascal as #alias; },
                quote! { #alias },
            )
        } else {
            (
                quote! { use ::domain::#domain_pascal; },
                quote! { #domain_pascal },
            )
        };

        let handler_doc = make_doc_comment(doc.description.as_deref());
        let dg_and_idem = format_delivery_and_idempotency_tokens(doc);

        let gen_content = match kind {
            IntegrationHandlerKind::EventHandler => {
                let deps_ident = format_ident!("{}Deps", handler_pascal);
                quote! {
                    use crate::handlers::#handler_snake::#deps_ident;
                    #domain_import
                    #(#parsed_handle_imports)*

                    #handler_doc
                    #[chromatofoor::event_handler(
                        scope = #domain_scope_ref,
                        handles = [#(#handle_event_idents),*],
                        #dg_and_idem
                    )]
                    #[derive(Clone, Default, Debug)]
                    pub struct #handler_pascal {
                        pub deps: #deps_ident,
                    }

                    impl std::ops::Deref for #handler_pascal {
                        type Target = #deps_ident;

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
                    #domain_import
                    #(#parsed_handle_imports)*
                    #(#parsed_emit_imports)*

                    #handler_doc
                    #[chromatofoor::policy(
                        scope = #domain_scope_ref,
                        handles = [#(#handle_event_idents),*],
                        emits = [#(#emit_command_idents),*],
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
            let mut sorted_manual_exports = std::collections::BTreeSet::new();
            sorted_manual_exports.insert(handler_pascal.to_string());
            for id in &handle_event_idents {
                sorted_manual_exports.insert(id.to_string());
            }

            let manual_content = match kind {
                IntegrationHandlerKind::EventHandler => {
                    let deps_ident = format_ident!("{}Deps", handler_pascal);
                    let manual_export_idents: Vec<_> = sorted_manual_exports
                        .iter()
                        .map(|n| format_ident!("{n}"))
                        .collect();
                    let eh_header = match doc.description.as_deref().map(str::trim) {
                        Some(d) if !d.is_empty() => {
                            format!("Event Handler: `{}` — {d}", doc.name)
                        }
                        _ => format!("Event Handler: `{}`", doc.name),
                    };
                    let handle_impls = handle_event_entries.iter().map(
                        |(ev_bc, ev_agg, ev_kebab, ev_pascal_str, ev_local_ident)| {
                            let mut doc_lines = vec![
                                eh_header.clone(),
                                format!("Trigger: `{ev_bc}` / `{ev_agg}` / `{ev_kebab}`"),
                            ];
                            let mut bullets = Vec::new();
                            if let Some(constraints) = &doc.constraints
                                && !constraints.is_empty()
                            {
                                bullets.push("- Constraints:".to_string());
                                for c in constraints {
                                    match c.rule.as_deref().map(str::trim) {
                                        Some(r) if !r.is_empty() => {
                                            bullets.push(format!("  - `{}` — {r}", c.name));
                                        }
                                        _ => bullets.push(format!("  - `{}`", c.name)),
                                    }
                                }
                            }
                            if let Some(effects) = &doc.side_effects
                                && !effects.is_empty()
                            {
                                bullets.push("- Side Effects:".to_string());
                                for eff in effects {
                                    let kind_label = match (
                                        eff.effect_type.as_deref(),
                                        eff.external_system.as_deref(),
                                    ) {
                                        (Some("external-call"), Some(sys)) => {
                                            format!("`external-call` (`{sys}`)")
                                        }
                                        (Some(t), _) => format!("`{t}`"),
                                        _ => "`side-effect`".to_string(),
                                    };
                                    match eff.rule.as_deref().map(str::trim) {
                                        Some(r) if !r.is_empty() => {
                                            bullets.push(format!("  - {kind_label} — {r}"));
                                        }
                                        _ => bullets.push(format!("  - {kind_label}")),
                                    }
                                }
                            }
                            if !bullets.is_empty() {
                                doc_lines.push(String::new());
                                doc_lines.extend(bullets);
                            }
                            let impl_doc = make_doc_lines(&doc_lines);
                            let todo_msg = format!(
                                "implement Handle::{ev_pascal_str}::handle for {handler_pascal}"
                            );
                            quote! {
                                #impl_doc
                                impl Handle<#ev_local_ident> for #handler_pascal {
                                    async fn handle(
                                        &self,
                                        _event: Envelope<#ev_local_ident>,
                                    ) -> Result<(), DomainError> {
                                        todo!(#todo_msg)
                                    }
                                }
                            }
                        },
                    );
                    quote! {
                        use crate::generated::handlers::#handler_snake::{
                            #(#manual_export_idents),*
                        };
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
                    sorted_manual_exports.insert(commands_enum_ident.to_string());
                    let manual_export_idents: Vec<_> = sorted_manual_exports
                        .iter()
                        .map(|n| format_ident!("{n}"))
                        .collect();
                    let policy_header = match doc.description.as_deref().map(str::trim) {
                        Some(d) if !d.is_empty() => format!("Policy: `{}` — {d}", doc.name),
                        _ => format!("Policy: `{}`", doc.name),
                    };
                    let react_impls = handle_event_entries.iter().map(
                        |(ev_bc, ev_agg, ev_kebab, ev_pascal_str, ev_local_ident)| {
                            let mut doc_lines = vec![
                                policy_header.clone(),
                                format!("Trigger: `{ev_bc}` / `{ev_agg}` / `{ev_kebab}`"),
                            ];
                            let mut bullets = Vec::new();
                            if let Some(emits) = &doc.emits
                                && !emits.is_empty()
                            {
                                let list = emits
                                    .iter()
                                    .filter_map(|e| {
                                        let c = e.command.as_deref()?;
                                        match (e.bounded_context.as_deref(), e.aggregate.as_deref())
                                        {
                                            (Some(b), Some(a)) => Some(format!("`{b}/{a}/{c}`")),
                                            _ => Some(format!("`{c}`")),
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                if !list.is_empty() {
                                    bullets.push(format!("- Emits: {list}"));
                                }
                            }
                            if let Some(constraints) = &doc.constraints
                                && !constraints.is_empty()
                            {
                                bullets.push("- Constraints:".to_string());
                                for c in constraints {
                                    match c.rule.as_deref().map(str::trim) {
                                        Some(r) if !r.is_empty() => {
                                            bullets.push(format!("  - `{}` — {r}", c.name));
                                        }
                                        _ => bullets.push(format!("  - `{}`", c.name)),
                                    }
                                }
                            }
                            if !bullets.is_empty() {
                                doc_lines.push(String::new());
                                doc_lines.extend(bullets);
                            }
                            let impl_doc = make_doc_lines(&doc_lines);
                            let todo_msg = format!(
                                "implement React::{ev_pascal_str}::react for {handler_pascal}"
                            );
                            quote! {
                                #impl_doc
                                impl React<#ev_local_ident> for #handler_pascal {
                                    fn react(
                                        &self,
                                        _event: Envelope<#ev_local_ident>,
                                    ) -> Vec<#commands_enum_ident> {
                                        todo!(#todo_msg)
                                    }
                                }
                            }
                        },
                    );
                    quote! {
                        use crate::generated::handlers::#handler_snake::{
                            #(#manual_export_idents),*
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
