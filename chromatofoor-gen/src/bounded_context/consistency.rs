use crate::common::GeneratorContext;
use crate::common::generate_struct_fields;
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_consistency_units(
    out_dir: &PathBuf,
    contexts: &HashMap<(String, String), GeneratorContext>,
    bc_contexts: &HashMap<String, crate::common::BoundedContextGenerator>,
    routing: &crate::common::RoutingContext,
) -> Result<()> {
    let mut sorted_contexts: Vec<_> = contexts.iter().collect();
    sorted_contexts.sort_by_key(|((bc, agg), _)| (bc.clone(), agg.clone()));

    for ((bc, agg), ctx) in sorted_contexts {
        if bc == "global" || agg == "global" {
            continue;
        }

        let bc_dir = crate::common::resolve_bc_dir(out_dir, bc, routing);
        let gen_base_path = bc_dir
            .join("src")
            .join("generated")
            .join("domain")
            .join(agg.to_case(Case::Snake));
        fs::create_dir_all(&gen_base_path)?;

        let manual_base_path = bc_dir
            .join("src")
            .join("domain")
            .join(agg.to_case(Case::Snake));

        let mut agg_mod_chunks = Vec::new();

        let agg_pascal = format_ident!("{}", agg.to_case(Case::Pascal));
        let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));

        let command_enum_ident = format_ident!("{}Command", agg_pascal);
        let error_ident = format_ident!("{}Error", agg_pascal);
        let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));

        // Resolve consulted + own events for Dynamic Consistency Boundaries in deterministic order
        let mut dcb_events: Vec<(String, esdm::EsdmDoc)> = Vec::new();
        if let Some(boundary_doc) = &ctx.boundary {
            let mut seen_dcb_events = std::collections::HashSet::new();
            for ev in &ctx.events {
                if seen_dcb_events.insert((agg.clone(), ev.name.clone())) {
                    dcb_events.push((agg.clone(), ev.clone()));
                }
            }
            if let Some(consults) = &boundary_doc.consults {
                let mut sorted_ctx_keys: Vec<_> = contexts.keys().collect();
                sorted_ctx_keys.sort();
                for c in consults {
                    if c.bounded_context.as_deref().is_some_and(|c_bc| c_bc != bc) {
                        continue;
                    }
                    let Some(ev_name) = &c.event else {
                        continue;
                    };
                    if let Some(c_agg) = &c.aggregate {
                        if let Some(other_ctx) = contexts.get(&(bc.clone(), c_agg.clone()))
                            && let Some(ev_doc) =
                                other_ctx.events.iter().find(|e| &e.name == ev_name)
                            && seen_dcb_events.insert((c_agg.clone(), ev_doc.name.clone()))
                        {
                            dcb_events.push((c_agg.clone(), ev_doc.clone()));
                        }
                    } else {
                        for (other_bc, other_unit) in &sorted_ctx_keys {
                            if other_bc == bc
                                && *other_unit != "global"
                                && let Some(other_ctx) =
                                    contexts.get(&((*other_bc).clone(), (*other_unit).clone()))
                                && let Some(ev_doc) =
                                    other_ctx.events.iter().find(|e| &e.name == ev_name)
                            {
                                if seen_dcb_events
                                    .insert(((*other_unit).clone(), ev_doc.name.clone()))
                                {
                                    dcb_events.push(((*other_unit).clone(), ev_doc.clone()));
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // --- Generate Commands (Generated Tree) ---
        {
            agg_mod_chunks.push(quote! { pub mod command; });
            let gen_command_dir = gen_base_path.join("command");
            fs::create_dir_all(&gen_command_dir)?;

            let mut gen_mod_rs_chunks = Vec::new();

            gen_mod_rs_chunks.push(quote! {
                use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
            });

            let mut command_mods = Vec::new();
            for cmd in &ctx.commands {
                let cmd_file_name = format_ident!("{}", cmd.name.to_case(Case::Snake));
                let struct_name = format_ident!("{}", cmd.name.to_case(Case::Pascal));
                command_mods
                    .push(quote! { pub mod #cmd_file_name; pub use #cmd_file_name::#struct_name; });
            }
            if !command_mods.is_empty() {
                gen_mod_rs_chunks.push(quote! { #(#command_mods)* });
            }

            let command_variants = ctx.commands.iter().map(|cmd| {
                let struct_name = format_ident!("{}", cmd.name.to_case(Case::Pascal));
                let mod_name = format_ident!("{}", cmd.name.to_case(Case::Snake));
                let cmd_doc = crate::common::make_doc_comment(cmd.description.as_deref());
                quote! {
                    #cmd_doc
                    #struct_name(#mod_name::#struct_name)
                }
            });

            gen_mod_rs_chunks.push(quote! {
                #[chromatofoor::command(
                    scope = #agg_pascal,
                    error = crate::generated::domain::#agg_snake::error::#error_ident,
                    validate,
                )]
                pub enum #command_enum_ident {
                    #(#command_variants),*
                }
            });

            for cmd in &ctx.commands {
                let cmd_file_name = format_ident!("{}", cmd.name.to_case(Case::Snake));
                let struct_name = format_ident!("{}", cmd.name.to_case(Case::Pascal));

                let mut cmd_code_chunks = Vec::new();

                cmd_code_chunks.push(quote! {
                    #[allow(unused_imports)]
                    use chromatofoor::validator::Validate;
                    #[allow(unused_imports)]
                    use serde::{Deserialize, Serialize};
                });

                let mut extra_structs = Vec::new();
                let mut fields = Vec::new();

                if let Some(data) = &cmd.data {
                    fields = crate::common::generate_struct_fields(
                        data,
                        &struct_name.to_string(),
                        &mut extra_structs,
                    );
                }

                let is_create = if let Some(lc) = &ctx.lifecycle {
                    lc.created
                        .as_ref()
                        .is_some_and(|list| list.iter().any(|c| c == &cmd.name))
                } else {
                    ctx.boundary.is_none()
                };

                for e in extra_structs {
                    cmd_code_chunks.push(e);
                }

                let cmd_doc = crate::common::make_doc_comment(cmd.description.as_deref());

                let mut actor_paths = Vec::new();
                if let Some(actors) = &cmd.actors {
                    let bc_actors = bc_contexts.get(bc).map(|b| &b.actors);
                    for a in actors {
                        let exists_in_bc =
                            bc_actors.is_some_and(|list| list.iter().any(|doc| &doc.name == a));
                        if !exists_in_bc {
                            anyhow::bail!(
                                "Command '{}' in bounded context '{}' references unknown actor '{}'",
                                cmd.name,
                                bc,
                                a
                            );
                        }
                        let actor_pascal = format_ident!("{}", a.to_case(Case::Pascal));
                        actor_paths.push(quote! {
                            crate::generated::domain::actors::#actor_pascal
                        });
                    }
                }

                let creates_tok = if is_create {
                    quote! { creates, }
                } else {
                    quote! {}
                };

                let cmd_attr = if actor_paths.is_empty() {
                    quote! {
                        #[chromatofoor::command(
                            scope = crate::generated::domain::#agg_snake::aggregate::#agg_pascal,
                            #creates_tok
                            validate,
                        )]
                    }
                } else {
                    quote! {
                        #[chromatofoor::command(
                            scope = crate::generated::domain::#agg_snake::aggregate::#agg_pascal,
                            #creates_tok
                            validate,
                            actors = [#(#actor_paths),*],
                        )]
                    }
                };

                cmd_code_chunks.push(quote! {
                    #cmd_doc
                    #cmd_attr
                    pub struct #struct_name {
                        #(#fields),*
                    }
                });

                let cmd_tokens = quote! { #(#cmd_code_chunks)* };
                crate::common::write_rust_file(
                    &gen_command_dir.join(format!("{cmd_file_name}.rs")),
                    cmd_tokens,
                )?;
            }

            let gen_mod_rs_tokens = quote! { #(#gen_mod_rs_chunks)* };
            crate::common::write_rust_file(&gen_command_dir.join("mod.rs"), gen_mod_rs_tokens)?;
        }

        // --- Generate Events (Generated Tree) ---
        {
            agg_mod_chunks.push(quote! { pub mod event; });
            let gen_event_dir = gen_base_path.join("event");
            fs::create_dir_all(&gen_event_dir)?;

            let mut gen_event_mod_chunks = Vec::new();

            if ctx.boundary.is_none() {
                gen_event_mod_chunks.push(quote! {
                    use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
                });
            }

            let mut event_mods = Vec::new();
            let mut exported_pascals = std::collections::HashSet::new();
            for ev in &ctx.events {
                let ev_file_name = format_ident!("{}", ev.name.to_case(Case::Snake));
                let pascal_str = ev.name.to_case(Case::Pascal);
                let struct_name = format_ident!("{}", pascal_str);
                exported_pascals.insert(pascal_str);
                event_mods
                    .push(quote! { pub mod #ev_file_name; pub use #ev_file_name::#struct_name; });
            }
            if !event_mods.is_empty() {
                gen_event_mod_chunks.push(quote! { #(#event_mods)* });
            }

            let event_enum_name = format_ident!("{}Event", agg_pascal);

            if ctx.boundary.is_some() {
                for (owner_unit, ev_doc) in &dcb_events {
                    if owner_unit != agg {
                        let pascal_str = ev_doc.name.to_case(Case::Pascal);
                        if exported_pascals.insert(pascal_str.clone()) {
                            let owner_snake = format_ident!("{}", owner_unit.to_case(Case::Snake));
                            let ev_mod = format_ident!("{}", ev_doc.name.to_case(Case::Snake));
                            let struct_name = format_ident!("{}", pascal_str);
                            gen_event_mod_chunks.push(quote! {
                                pub use crate::generated::domain::#owner_snake::event::#ev_mod::#struct_name;
                            });
                        }
                    }
                }
                gen_event_mod_chunks.push(quote! {
                    pub use crate::generated::domain::#agg_snake::aggregate::#event_enum_name;
                });
            } else {
                let event_variants = ctx.events.iter().map(|ev| {
                    let struct_name = format_ident!("{}", ev.name.to_case(Case::Pascal));
                    let mod_name = format_ident!("{}", ev.name.to_case(Case::Snake));
                    let ev_doc = crate::common::make_doc_comment(ev.description.as_deref());
                    quote! {
                        #ev_doc
                        #struct_name(#mod_name::#struct_name)
                    }
                });

                gen_event_mod_chunks.push(quote! {
                    #[chromatofoor::event(scope = #agg_pascal)]
                    pub enum #event_enum_name {
                        #(#event_variants),*
                    }
                });
            }

            for ev in &ctx.events {
                let ev_file_name = format_ident!("{}", ev.name.to_case(Case::Snake));
                let struct_name = format_ident!("{}", ev.name.to_case(Case::Pascal));

                let mut ev_code_chunks = Vec::new();

                ev_code_chunks.push(quote! {
                    #[allow(unused_imports)]
                    use chromatofoor::validator::Validate;
                    #[allow(unused_imports)]
                    use serde::{Deserialize, Serialize};
                });

                let mut extra_structs = Vec::new();
                let mut fields = Vec::new();
                if let Some(data) = &ev.data {
                    fields =
                        generate_struct_fields(data, &struct_name.to_string(), &mut extra_structs);
                }

                for e in extra_structs {
                    ev_code_chunks.push(e);
                }

                let ev_doc = crate::common::make_doc_comment(ev.description.as_deref());
                let ev_scope_ty = if ctx.boundary.is_some() {
                    quote! { crate::generated::domain::#bc_pascal }
                } else {
                    quote! { crate::generated::domain::#agg_snake::aggregate::#agg_pascal }
                };

                ev_code_chunks.push(quote! {
                    #ev_doc
                    #[chromatofoor::event(scope = #ev_scope_ty)]
                    #[derive(chromatofoor::validator::Validate)]
                    #[validate(crate = "chromatofoor::validator")]
                    pub struct #struct_name {
                        #(#fields),*
                    }
                });

                let ev_tokens = quote! { #(#ev_code_chunks)* };
                crate::common::write_rust_file(
                    &gen_event_dir.join(format!("{ev_file_name}.rs")),
                    ev_tokens,
                )?;
            }

            let gen_event_mod_tokens = quote! { #(#gen_event_mod_chunks)* };
            crate::common::write_rust_file(&gen_event_dir.join("mod.rs"), gen_event_mod_tokens)?;
        }

        // --- Generate aggregate.rs (Generated Tree) ---
        let inner_struct = format_ident!("{}Inner", agg_pascal);
        let dcb_state_ident = format_ident!("{}State", agg_pascal);

        let mut agg_code_chunks = Vec::new();

        let mut id_attr_specs = Vec::new();
        let id_spec = ctx
            .aggregate
            .as_ref()
            .and_then(|a| a.identified_by.as_ref())
            .or_else(|| ctx.boundary.as_ref().and_then(|b| b.identified_by.as_ref()));
        if let Some(spec) = id_spec {
            for entry in spec.as_slice() {
                match entry.source.as_str() {
                    "state" => {
                        let f = entry.field.as_deref().unwrap_or("id").to_case(Case::Snake);
                        let f_ident = format_ident!("{}", f);
                        id_attr_specs.push(quote! { state(#f_ident) });
                    }
                    "command-payload" => {
                        let f = entry.field.as_deref().unwrap_or("id").to_case(Case::Snake);
                        let f_ident = format_ident!("{}", f);
                        id_attr_specs.push(quote! { command_payload(#f_ident) });
                    }
                    "schema" => {
                        let f = entry.field.as_deref().unwrap_or("id").to_case(Case::Snake);
                        let f_ident = format_ident!("{}", f);
                        id_attr_specs.push(quote! { schema(#f_ident) });
                    }
                    "static" => {
                        let v = entry.value.as_deref().unwrap_or("singleton");
                        id_attr_specs.push(quote! { static(#v) });
                    }
                    _ => {
                        let g = entry.generator.as_deref().unwrap_or("uuid");
                        let g_ident = format_ident!("{}", g.to_case(Case::Snake));
                        id_attr_specs.push(quote! { generated(#g_ident) });
                    }
                }
            }
        }
        if id_attr_specs.is_empty() {
            id_attr_specs.push(quote! { generated(uuid) });
        }

        agg_code_chunks.push(quote! {
            #[allow(unused_imports)]
            use chromatofoor::validator::Validate;
            use serde::{Deserialize, Serialize};
        });

        let agg_doc = ctx
            .aggregate
            .as_ref()
            .and_then(|a| a.description.as_deref())
            .or_else(|| ctx.boundary.as_ref().and_then(|b| b.description.as_deref()))
            .map_or(quote! {}, |d| crate::common::make_doc_comment(Some(d)));

        if ctx.boundary.is_none() {
            agg_code_chunks.push(quote! {
                #agg_doc
                #[chromatofoor::aggregate(
                    scope = crate::generated::domain::#bc_pascal,
                    identified_by = [#(#id_attr_specs),*],
                )]
                #[derive(Default, Deserialize, Serialize, Debug, Clone)]
                pub enum #agg_pascal {
                    #[default]
                    None,
                    Created(#inner_struct),
                }
            });

            let mut inner_struct_fields = Vec::new();
            let mut extra_structs = Vec::new();

            if let Some(agg_doc) = &ctx.aggregate
                && let Some(state) = &agg_doc.state
            {
                inner_struct_fields =
                    generate_struct_fields(state, &agg_pascal.to_string(), &mut extra_structs);
            }

            for extra in extra_structs {
                agg_code_chunks.push(extra);
            }

            agg_code_chunks.push(quote! {
                #[derive(Deserialize, Serialize, Debug, Clone, PartialEq, chromatofoor::validator::Validate)]
                #[validate(crate = "chromatofoor::validator")]
                pub struct #inner_struct {
                    #(#inner_struct_fields),*
                }
            });

            agg_code_chunks.push(quote! {
                impl #agg_pascal {
                    pub fn is_none(&self) -> bool {
                        matches!(self, Self::None)
                    }
                }
            });
        } else {
            let consult_paths: Vec<_> = dcb_events
                .iter()
                .map(|(owner_unit, ev_doc)| {
                    let owner_snake = format_ident!("{}", owner_unit.to_case(Case::Snake));
                    let ev_mod = format_ident!("{}", ev_doc.name.to_case(Case::Snake));
                    let ev_struct = format_ident!("{}", ev_doc.name.to_case(Case::Pascal));
                    quote! {
                        crate::generated::domain::#owner_snake::event::#ev_mod::#ev_struct
                    }
                })
                .collect();

            agg_code_chunks.push(quote! {
                #agg_doc
                #[chromatofoor::dynamic_consistency_boundary(
                    scope = crate::generated::domain::#bc_pascal,
                    identified_by = [#(#id_attr_specs),*],
                    consults = [#(#consult_paths),*],
                )]
                #[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq)]
                pub struct #agg_pascal {
                    pub state: crate::domain::#agg_snake::aggregate::#dcb_state_ident,
                }

                impl std::ops::Deref for #agg_pascal {
                    type Target = crate::domain::#agg_snake::aggregate::#dcb_state_ident;

                    fn deref(&self) -> &Self::Target {
                        &self.state
                    }
                }

                impl std::ops::DerefMut for #agg_pascal {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.state
                    }
                }
            });
        }

        let agg_tokens = quote! { #(#agg_code_chunks)* };
        crate::common::write_rust_file(&gen_base_path.join("aggregate.rs"), agg_tokens)?;
        agg_mod_chunks.push(quote! { pub mod aggregate; });

        // --- Generate error.rs (Generated Tree) ---
        let mut err_code_chunks = Vec::new();

        let mut invariants = Vec::new();
        if let Some(ref agg_doc) = ctx.aggregate
            && let Some(ref invs) = agg_doc.invariants
        {
            invariants.extend(invs.clone());
        }
        if let Some(ref boundary_doc) = ctx.boundary
            && let Some(ref invs) = boundary_doc.invariants
        {
            invariants.extend(invs.clone());
        }

        let error_variants = invariants.iter().map(|inv| {
            let variant_ident = format_ident!("{}", inv.name.to_case(Case::Pascal));
            let name_str = &inv.name;
            quote! {
                #[error(#name_str)]
                #variant_ident
            }
        });

        err_code_chunks.push(quote! {
            #[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
            pub enum #error_ident {
                #(#error_variants,)*
                #[error("{0}")]
                Validation(String),
            }
        });

        err_code_chunks.push(quote! {
            impl chromatofoor::architecturekit::Categorized for #error_ident {
                fn category(&self) -> Option<chromatofoor::architecturekit::Category> {
                    Some(chromatofoor::architecturekit::Category::Domain)
                }
            }

            impl From<chromatofoor::validator::ValidationErrors> for #error_ident {
                fn from(err: chromatofoor::validator::ValidationErrors) -> Self {
                    Self::Validation(err.to_string())
                }
            }
        });

        let err_tokens = quote! { #(#err_code_chunks)* };
        crate::common::write_rust_file(&gen_base_path.join("error.rs"), err_tokens)?;
        agg_mod_chunks.push(quote! { pub mod error; });

        let agg_mod_tokens = quote! { #(#agg_mod_chunks)* };
        crate::common::write_rust_file(&gen_base_path.join("mod.rs"), agg_mod_tokens)?;

        // ==========================================
        // --- SCAFFOLD MANUAL IMPLEMENTATIONS ------
        // ==========================================

        if !manual_base_path.exists() {
            fs::create_dir_all(&manual_base_path)?;
        }

        // 1. Scaffold manual mod.rs if missing or update
        let manual_mod_rs = manual_base_path.join("mod.rs");
        let mut agg_manual_mods = vec!["aggregate".to_string()];
        if !ctx.commands.is_empty() {
            agg_manual_mods.push("command".to_string());
        }
        let has_manual_events = if ctx.boundary.is_some() {
            !dcb_events.is_empty()
        } else {
            !ctx.events.is_empty()
        };
        if has_manual_events {
            agg_manual_mods.push("event".to_string());
        }
        crate::common::ensure_manual_mod_file(&manual_mod_rs, &agg_manual_mods)?;

        // 2. Scaffold manual aggregate.rs if missing
        let manual_aggregate_rs = manual_base_path.join("aggregate.rs");
        if !manual_aggregate_rs.exists() {
            if ctx.boundary.is_some() {
                crate::common::write_manual_rust_file(
                    &manual_aggregate_rs,
                    quote! {
                        use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
                        use serde::{Deserialize, Serialize};

                        #[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq)]
                        pub struct #dcb_state_ident {}

                        impl #agg_pascal {}
                    },
                )?;
            } else {
                crate::common::write_manual_rust_file(
                    &manual_aggregate_rs,
                    quote! {
                        use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;

                        impl #agg_pascal {}
                    },
                )?;
            }
        }

        // 3. Scaffold manual command/ directory if missing
        if !ctx.commands.is_empty() {
            let manual_command_dir = manual_base_path.join("command");
            if !manual_command_dir.exists() {
                fs::create_dir_all(&manual_command_dir)?;
            }

            let manual_command_mod = manual_command_dir.join("mod.rs");
            let manual_command_mods: Vec<String> = ctx
                .commands
                .iter()
                .map(|cmd| cmd.name.to_case(Case::Snake))
                .collect();
            crate::common::ensure_manual_mod_file(&manual_command_mod, &manual_command_mods)?;

            let agg_error_ident = format_ident!("{}Error", agg_pascal);
            let agg_event_ident = format_ident!("{}Event", agg_pascal);
            for cmd in &ctx.commands {
                let cmd_file_name = cmd.name.to_case(Case::Snake);
                let struct_ident = format_ident!("{}", cmd.name.to_case(Case::Pascal));
                let manual_cmd_file = manual_command_dir.join(format!("{cmd_file_name}.rs"));

                if !manual_cmd_file.exists() {
                    let todo_msg =
                        format!("implement Command::{agg_pascal}::decide for {struct_ident}");
                    crate::common::write_manual_rust_file(
                        &manual_cmd_file,
                        quote! {
                            use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
                            use crate::generated::domain::#agg_snake::command::#struct_ident;
                            use crate::generated::domain::#agg_snake::error::#agg_error_ident;
                            use crate::generated::domain::#agg_snake::event::*;
                            use chromatofoor::bounded_context::Command;

                            impl Command<#agg_pascal> for #struct_ident {
                                type Error = #agg_error_ident;

                                #[tracing::instrument(skip(self, _state))]
                                async fn decide(
                                    self,
                                    _state: &#agg_pascal,
                                ) -> Result<Vec<#agg_event_ident>, Self::Error> {
                                    todo!(#todo_msg)
                                }
                            }
                        },
                    )?;
                }
            }
        }

        // 5. Scaffold manual event/ directory if missing
        if ctx.boundary.is_some() {
            if !dcb_events.is_empty() {
                let manual_event_dir = manual_base_path.join("event");
                if !manual_event_dir.exists() {
                    fs::create_dir_all(&manual_event_dir)?;
                }

                let mut ev_name_counts: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for (_, ev) in &dcb_events {
                    *ev_name_counts
                        .entry(ev.name.to_case(Case::Snake))
                        .or_insert(0) += 1;
                }

                let mut manual_event_mods = Vec::new();
                for (owner_unit, ev) in &dcb_events {
                    let ev_snake = ev.name.to_case(Case::Snake);
                    let file_mod_name = if ev_name_counts.get(&ev_snake).copied().unwrap_or(0) > 1 {
                        format!("{}_{}", owner_unit.to_case(Case::Snake), ev_snake)
                    } else {
                        ev_snake.clone()
                    };
                    manual_event_mods.push(file_mod_name.clone());

                    let owner_snake = format_ident!("{}", owner_unit.to_case(Case::Snake));
                    let ev_mod = format_ident!("{}", ev_snake);
                    let struct_ident = format_ident!("{}", ev.name.to_case(Case::Pascal));
                    let manual_ev_file = manual_event_dir.join(format!("{file_mod_name}.rs"));

                    if !manual_ev_file.exists() {
                        let todo_msg = format!(
                            "implement Event::{agg_pascal}::evolve for {struct_ident} (return _state if stateless)"
                        );
                        crate::common::write_manual_rust_file(
                            &manual_ev_file,
                            quote! {
                                use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
                                use crate::generated::domain::#owner_snake::event::#ev_mod::#struct_ident;
                                use chromatofoor::bounded_context::Event;

                                impl Event<#agg_pascal> for #struct_ident {
                                    fn evolve(&self, _state: #agg_pascal) -> #agg_pascal {
                                        todo!(#todo_msg)
                                    }
                                }
                            },
                        )?;
                    }
                }

                let manual_event_mod = manual_event_dir.join("mod.rs");
                crate::common::ensure_manual_mod_file(&manual_event_mod, &manual_event_mods)?;
            }
        } else if !ctx.events.is_empty() {
            let manual_event_dir = manual_base_path.join("event");
            if !manual_event_dir.exists() {
                fs::create_dir_all(&manual_event_dir)?;
            }

            let manual_event_mod = manual_event_dir.join("mod.rs");
            let manual_event_mods: Vec<String> = ctx
                .events
                .iter()
                .map(|ev| ev.name.to_case(Case::Snake))
                .collect();
            crate::common::ensure_manual_mod_file(&manual_event_mod, &manual_event_mods)?;

            for ev in &ctx.events {
                let ev_file_name = ev.name.to_case(Case::Snake);
                let struct_ident = format_ident!("{}", ev.name.to_case(Case::Pascal));
                let manual_ev_file = manual_event_dir.join(format!("{ev_file_name}.rs"));

                if !manual_ev_file.exists() {
                    let todo_msg = format!(
                        "implement Event::{agg_pascal}::evolve for {struct_ident} (return _state if stateless)"
                    );
                    crate::common::write_manual_rust_file(
                        &manual_ev_file,
                        quote! {
                            use crate::generated::domain::#agg_snake::aggregate::#agg_pascal;
                            use crate::generated::domain::#agg_snake::event::#struct_ident;
                            use chromatofoor::bounded_context::Event;

                            impl Event<#agg_pascal> for #struct_ident {
                                fn evolve(&self, _state: #agg_pascal) -> #agg_pascal {
                                    todo!(#todo_msg)
                                }
                            }
                        },
                    )?;
                }
            }
        }
    }

    // Generate BC-scoped models
    let mut sorted_bc_contexts: Vec<_> = bc_contexts.iter().collect();
    sorted_bc_contexts.sort_by_key(|(k, _)| *k);
    for (bc, bc_ctx) in sorted_bc_contexts {
        if bc == "global" {
            continue;
        }
        let bc_dir = crate::common::resolve_bc_dir(out_dir, bc, routing);
        let bc_gen_base_path = bc_dir.join("src").join("generated").join("domain");

        let mut bc_value_objects: Vec<esdm::EsdmDoc> = bc_ctx.value_objects.clone();
        let mut bc_entities: Vec<esdm::EsdmDoc> = bc_ctx.entities.clone();
        let mut sorted_contexts: Vec<_> = contexts.iter().collect();
        sorted_contexts.sort_by_key(|((c_bc, c_agg), _)| (c_bc.clone(), c_agg.clone()));
        for ((ctx_bc, _agg), ctx) in sorted_contexts {
            if ctx_bc == bc {
                bc_value_objects.extend(ctx.value_objects.clone());
                bc_entities.extend(ctx.entities.clone());
            }
        }
        generate_schema_structs(
            &bc_value_objects,
            &bc_gen_base_path.join("value_objects"),
            bc,
            false,
        )?;
        generate_schema_structs(&bc_entities, &bc_gen_base_path.join("entities"), bc, true)?;
        generate_actors(&bc_ctx.actors, &bc_gen_base_path, bc)?;
    }

    Ok(())
}

pub fn generate_actors(
    actors: &[esdm::EsdmDoc],
    out_dir: &std::path::PathBuf,
    bc: &str,
) -> anyhow::Result<()> {
    if actors.is_empty() {
        return Ok(());
    }

    let actors_dir = out_dir.join("actors");
    std::fs::create_dir_all(&actors_dir)?;

    let bc_pascal = quote::format_ident!(
        "{}",
        convert_case::Casing::to_case(&bc, convert_case::Case::Pascal)
    );
    let mut actor_structs = Vec::new();

    for actor in actors {
        let actor_pascal = quote::format_ident!(
            "{}",
            convert_case::Casing::to_case(&actor.name, convert_case::Case::Pascal)
        );
        let actor_doc = crate::common::make_doc_comment(actor.description.as_deref());
        let is_system = actor.doc_type.as_deref() == Some("system");
        let type_ident = if is_system {
            quote::format_ident!("system")
        } else {
            quote::format_ident!("human")
        };

        actor_structs.push(quote::quote! {
            #actor_doc
            #[chromatofoor::actor(
                scope = crate::generated::domain::#bc_pascal,
                type = #type_ident,
            )]
            pub struct #actor_pascal;
        });
    }

    let actors_code = quote::quote! {
        #(#actor_structs)*
    };

    crate::common::write_rust_file(&actors_dir.join("mod.rs"), actors_code)?;

    Ok(())
}

pub fn generate_schema_structs(
    models: &[esdm::EsdmDoc],
    target_dir: &PathBuf,
    bc: &str,
    is_entity: bool,
) -> Result<()> {
    if models.is_empty() {
        return Ok(());
    }

    fs::create_dir_all(target_dir)?;

    let mut sorted_models: Vec<_> = models.iter().collect();
    sorted_models.sort_by_key(|m| m.name.clone());

    let mut mod_rs_chunks = Vec::new();
    let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));

    for model in sorted_models {
        let model_snake = format_ident!("{}", model.name.to_case(Case::Snake));
        let model_pascal = format_ident!("{}", model.name.to_case(Case::Pascal));

        mod_rs_chunks.push(quote! {
            pub mod #model_snake;
            pub use #model_snake::#model_pascal;
        });

        let schema = if let Some(schema) = &model.schema {
            schema
        } else if let Some(data) = &model.data {
            data
        } else {
            continue;
        };

        let mut extra_structs = Vec::new();
        let fields = crate::common::generate_struct_fields(
            schema,
            &model.name.to_case(Case::Pascal),
            &mut extra_structs,
        );

        let model_doc = crate::common::make_doc_comment(model.description.as_deref());

        let attr_tok = if is_entity {
            let mut id_specs = Vec::new();
            if let Some(spec) = &model.identified_by {
                for entry in spec.as_slice() {
                    match entry.source.as_str() {
                        "static" => {
                            let v = entry.value.as_deref().unwrap_or("singleton");
                            id_specs.push(quote! { static(#v) });
                        }
                        _ => {
                            let f = entry.field.as_deref().unwrap_or("id").to_case(Case::Snake);
                            let f_ident = format_ident!("{}", f);
                            id_specs.push(quote! { schema(#f_ident) });
                        }
                    }
                }
            }
            if id_specs.is_empty() {
                id_specs.push(quote! { schema(id) });
            }
            quote! {
                #[chromatofoor::entity(
                    scope = crate::generated::domain::#bc_pascal,
                    identified_by = [#(#id_specs),*],
                )]
            }
        } else {
            quote! {
                #[chromatofoor::value_object(scope = crate::generated::domain::#bc_pascal)]
            }
        };

        let model_struct = quote! {
            #[allow(unused_imports)]
            use chromatofoor::validator::Validate;
            #[allow(unused_imports)]
            use serde::{Serialize, Deserialize};

            #(#extra_structs)*

            #model_doc
            #attr_tok
            #[derive(chromatofoor::validator::Validate)]
            #[validate(crate = "chromatofoor::validator")]
            pub struct #model_pascal {
                #(#fields),*
            }
        };

        crate::common::write_rust_file(
            &target_dir.join(format!("{}.rs", model.name.to_case(Case::Snake))),
            model_struct,
        )?;
    }

    let mod_tokens = quote! { #(#mod_rs_chunks)* };
    crate::common::write_rust_file(&target_dir.join("mod.rs"), mod_tokens)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::*;
    use insta::assert_snapshot;

    #[test]
    fn test_domain_model_generation() {
        use crate::common::{BoundedContextGenerator, GeneratorContext};

        let aggregate = EsdmDoc {
            name: "TestAggregate".to_string(),
            kind: "aggregate".to_string(),
            ..Default::default()
        };

        let command = EsdmDoc {
            name: "CreateTest".to_string(),
            kind: "command".to_string(),
            data: Some(DataSchema {
                properties: indexmap::IndexMap::from([(
                    "id".to_string(),
                    PropertySchema {
                        prop_type: Some("string".to_string()),
                        format: Some("uuid".to_string()),
                        ..Default::default()
                    },
                )]),
                required: vec!["id".to_string()],
                example: None,
            }),
            ..Default::default()
        };

        let event = EsdmDoc {
            name: "TestCreated".to_string(),
            kind: "event".to_string(),
            data: Some(DataSchema {
                properties: indexmap::IndexMap::from([(
                    "id".to_string(),
                    PropertySchema {
                        prop_type: Some("string".to_string()),
                        format: Some("uuid".to_string()),
                        ..Default::default()
                    },
                )]),
                required: vec!["id".to_string()],
                example: None,
            }),
            ..Default::default()
        };

        let mut contexts = std::collections::HashMap::new();
        contexts.insert(
            ("TestContext".to_string(), "TestAggregate".to_string()),
            GeneratorContext {
                commands: vec![command],
                events: vec![event],
                aggregate: Some(aggregate),
                boundary: None,
                lifecycle: None,
                features: vec![],
                entities: vec![],
                value_objects: vec![],
            },
        );

        let bc_contexts: std::collections::HashMap<String, BoundedContextGenerator> =
            std::collections::HashMap::new();

        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = crate::common::RoutingContext::default();
        generate_consistency_units(&out_dir, &contexts, &bc_contexts, &routing).unwrap();

        let bc_dir = crate::common::resolve_bc_dir(&out_dir, "TestContext", &routing);
        let gen_dir = bc_dir.join("src/generated/domain/test_aggregate");

        let agg_rs = std::fs::read_to_string(gen_dir.join("aggregate.rs")).unwrap_or_default();
        let cmd_rs = std::fs::read_to_string(gen_dir.join("command/mod.rs")).unwrap_or_default();
        let cmd_struct_rs =
            std::fs::read_to_string(gen_dir.join("command/create_test.rs")).unwrap_or_default();
        let ev_rs = std::fs::read_to_string(gen_dir.join("event/mod.rs")).unwrap_or_default();
        let ev_struct_rs =
            std::fs::read_to_string(gen_dir.join("event/test_created.rs")).unwrap_or_default();
        let err_rs = std::fs::read_to_string(gen_dir.join("error.rs")).unwrap_or_default();

        let all_code = format!(
            "AGGREGATE:\n{agg_rs}\n\nCOMMAND:\n{cmd_rs}\n\nCOMMAND_STRUCT:\n{cmd_struct_rs}\n\nEVENT:\n{ev_rs}\n\nEVENT_STRUCT:\n{ev_struct_rs}\n\nERROR:\n{err_rs}"
        );
        assert_snapshot!("domain_generation", all_code);
    }
}
