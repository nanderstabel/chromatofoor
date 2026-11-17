use esdm::EsdmDoc;
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

type ScopedRef = (String, String, String);
type TimerEffects = (Vec<String>, Vec<String>);

pub(crate) struct ResolvedPmSpec {
    pub event_imports: BTreeSet<String>,
    pub command_imports: BTreeSet<String>,
    pub starts_when_idents: Vec<syn::Ident>,
    pub handles_idents: Vec<syn::Ident>,
    pub emit_cmd_idents: Vec<syn::Ident>,
    pub timer_specs: Vec<(syn::Ident, proc_macro2::TokenStream)>,
    pub has_ends_when: bool,
    pub event_ident_by_key: HashMap<ScopedRef, syn::Ident>,
    pub event_ident_by_event_name: HashMap<String, syn::Ident>,
    pub merged_timer_effects_by_event: HashMap<syn::Ident, (Vec<syn::Ident>, Vec<syn::Ident>)>,
    pub merged_timer_effects_by_timer: HashMap<syn::Ident, (Vec<syn::Ident>, Vec<syn::Ident>)>,
}

pub(crate) fn resolve_pm_event_bc<'a>(
    pm: &'a EsdmDoc,
    explicit_bc: Option<&'a str>,
    agg: &str,
    ev: &str,
) -> &'a str {
    explicit_bc
        .or_else(|| {
            pm.starts_when.as_ref().and_then(|sw| {
                sw.iter()
                    .find(|s| s.aggregate.as_deref() == Some(agg) || s.event.as_deref() == Some(ev))
                    .and_then(|s| s.bounded_context.as_deref())
            })
        })
        .unwrap_or_else(|| {
            pm.scope
                .as_ref()
                .and_then(|s| s.bounded_context.as_deref())
                .unwrap_or("global")
        })
}

pub(crate) fn resolve_pm_spec(pm: &EsdmDoc) -> ResolvedPmSpec {
    let mut distinct_cmds_by_pascal: HashMap<String, BTreeSet<(String, String, String)>> =
        HashMap::new();
    let mut distinct_events_by_pascal: HashMap<String, BTreeSet<(String, String, String)>> =
        HashMap::new();

    let mut raw_starter_events: Vec<(String, String, String)> = Vec::new();
    let mut raw_reaction_events: Vec<(String, String, String)> = Vec::new();
    let mut raw_emits: Vec<(String, String, String)> = Vec::new();

    if let Some(starts_when) = &pm.starts_when {
        for sw in starts_when {
            let agg = sw.aggregate.as_deref().unwrap_or("global");
            let ev = sw.event.as_deref().unwrap_or("global");
            let bc = resolve_pm_event_bc(pm, sw.bounded_context.as_deref(), agg, ev);
            if bc != "global" && agg != "global" && ev != "global" {
                let key = (bc.to_string(), agg.to_string(), ev.to_string());
                distinct_events_by_pascal
                    .entry(ev.to_case(Case::Pascal))
                    .or_default()
                    .insert(key.clone());
                if !raw_starter_events.contains(&key) {
                    raw_starter_events.push(key);
                }
            }
        }
    }

    let mut raw_timer_effects_by_event: HashMap<ScopedRef, TimerEffects> = HashMap::new();
    let mut raw_timer_effects_by_timer: HashMap<String, TimerEffects> = HashMap::new();
    let mut referenced_timer_kebabs: BTreeSet<String> = BTreeSet::new();

    if let Some(reactions) = &pm.reactions {
        for reaction in reactions {
            if let Some(when) = &reaction.when {
                if let Some(emits) = &reaction.emits {
                    for e in emits {
                        let target_bc = e.bounded_context.as_deref().unwrap_or("global");
                        let target_agg = e.aggregate.as_deref().unwrap_or("global");
                        let target_cmd = e.command.as_deref().unwrap_or("global");
                        if target_bc == "global" || target_agg == "global" || target_cmd == "global"
                        {
                            continue;
                        }
                        let key = (
                            target_bc.to_string(),
                            target_agg.to_string(),
                            target_cmd.to_string(),
                        );
                        distinct_cmds_by_pascal
                            .entry(target_cmd.to_case(Case::Pascal))
                            .or_default()
                            .insert(key.clone());
                        if !raw_emits.contains(&key) {
                            raw_emits.push(key);
                        }
                    }
                }

                if let Some(timer_name) = &when.timer {
                    let timer_kebab = timer_name.to_case(Case::Kebab);
                    referenced_timer_kebabs.insert(timer_kebab.clone());
                    let entry = raw_timer_effects_by_timer.entry(timer_kebab).or_default();
                    if let Some(set_timers) = &reaction.set_timers {
                        for t_name in set_timers {
                            let t_kebab = t_name.to_case(Case::Kebab);
                            referenced_timer_kebabs.insert(t_kebab.clone());
                            if !entry.0.contains(&t_kebab) {
                                entry.0.push(t_kebab);
                            }
                        }
                    }
                    if let Some(cancel_timers) = &reaction.cancel_timers {
                        for t_name in cancel_timers {
                            let t_kebab = t_name.to_case(Case::Kebab);
                            referenced_timer_kebabs.insert(t_kebab.clone());
                            if !entry.1.contains(&t_kebab) {
                                entry.1.push(t_kebab);
                            }
                        }
                    }
                } else {
                    let agg = when.aggregate.as_deref().unwrap_or("global");
                    let ev = when.event.as_deref().unwrap_or("global");
                    let bc = resolve_pm_event_bc(pm, when.bounded_context.as_deref(), agg, ev);
                    if bc != "global" && agg != "global" && ev != "global" {
                        let key = (bc.to_string(), agg.to_string(), ev.to_string());
                        distinct_events_by_pascal
                            .entry(ev.to_case(Case::Pascal))
                            .or_default()
                            .insert(key.clone());
                        if !raw_reaction_events.contains(&key) {
                            raw_reaction_events.push(key.clone());
                        }
                        let entry = raw_timer_effects_by_event.entry(key).or_default();
                        if let Some(set_timers) = &reaction.set_timers {
                            for t_name in set_timers {
                                let t_kebab = t_name.to_case(Case::Kebab);
                                referenced_timer_kebabs.insert(t_kebab.clone());
                                if !entry.0.contains(&t_kebab) {
                                    entry.0.push(t_kebab);
                                }
                            }
                        }
                        if let Some(cancel_timers) = &reaction.cancel_timers {
                            for t_name in cancel_timers {
                                let t_kebab = t_name.to_case(Case::Kebab);
                                referenced_timer_kebabs.insert(t_kebab.clone());
                                if !entry.1.contains(&t_kebab) {
                                    entry.1.push(t_kebab);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If `starts_when` was omitted or empty, treat all reaction events as starters.
    if raw_starter_events.is_empty() {
        raw_starter_events.clone_from(&raw_reaction_events);
    }

    let mut all_events = raw_starter_events.clone();
    for key in &raw_reaction_events {
        if !all_events.contains(key) {
            all_events.push(key.clone());
        }
    }

    let mut event_imports = BTreeSet::new();
    let mut event_ident_by_key = HashMap::new();
    let mut event_ident_by_event_name = HashMap::new();

    for (bc, agg, ev) in &all_events {
        let bc_mod = format_ident!("{}", bc.to_case(Case::Snake));
        let agg_snake = format_ident!("{}", agg.to_case(Case::Snake));
        let ev_pascal_str = ev.to_case(Case::Pascal);
        let ev_pascal = format_ident!("{}", ev_pascal_str);

        let has_collision = distinct_events_by_pascal
            .get(&ev_pascal_str)
            .is_some_and(|set| set.len() > 1);

        let ev_ident = if has_collision {
            let alias_ident = format_ident!(
                "{}{}{}",
                bc.to_case(Case::Pascal),
                agg.to_case(Case::Pascal),
                ev_pascal_str
            );
            event_imports.insert(
                quote! {
                    pub use #bc_mod::generated::domain::#agg_snake::event::#ev_pascal as #alias_ident;
                }
                .to_string(),
            );
            alias_ident
        } else {
            event_imports.insert(
                quote! {
                    pub use #bc_mod::generated::domain::#agg_snake::event::#ev_pascal;
                }
                .to_string(),
            );
            ev_pascal
        };

        event_ident_by_key.insert((bc.clone(), agg.clone(), ev.clone()), ev_ident.clone());
        event_ident_by_event_name
            .entry(ev.clone())
            .or_insert_with(|| ev_ident.clone());
    }

    let starts_when_idents: Vec<syn::Ident> = raw_starter_events
        .iter()
        .filter_map(|k| event_ident_by_key.get(k).cloned())
        .collect();

    let handles_idents: Vec<syn::Ident> = raw_reaction_events
        .iter()
        .filter(|k| !raw_starter_events.contains(k))
        .filter_map(|k| event_ident_by_key.get(k).cloned())
        .collect();

    let mut command_imports = BTreeSet::new();
    let mut emit_cmd_idents = Vec::new();

    for (target_bc, target_agg, target_cmd) in &raw_emits {
        let target_bc_mod = format_ident!("{}", target_bc.to_case(Case::Snake));
        let target_agg_snake = format_ident!("{}", target_agg.to_case(Case::Snake));
        let target_cmd_pascal = target_cmd.to_case(Case::Pascal);
        let target_cmd_struct = format_ident!("{}", target_cmd_pascal);
        let target_cmd_module = format_ident!("{}", target_cmd.to_case(Case::Snake));

        let has_collision = distinct_cmds_by_pascal
            .get(&target_cmd_pascal)
            .is_some_and(|set| set.len() > 1);

        let cmd_ident = if has_collision {
            let alias_ident = format_ident!(
                "{}{}{}",
                target_bc.to_case(Case::Pascal),
                target_agg.to_case(Case::Pascal),
                target_cmd_pascal
            );
            command_imports.insert(
                quote! {
                    pub use #target_bc_mod::generated::domain::#target_agg_snake::command::#target_cmd_module::#target_cmd_struct as #alias_ident;
                }
                .to_string(),
            );
            alias_ident
        } else {
            command_imports.insert(
                quote! {
                    pub use #target_bc_mod::generated::domain::#target_agg_snake::command::#target_cmd_module::#target_cmd_struct;
                }
                .to_string(),
            );
            target_cmd_struct
        };

        emit_cmd_idents.push(cmd_ident);
    }

    let mut declared_timers: BTreeMap<String, (syn::Ident, proc_macro2::TokenStream)> =
        BTreeMap::new();
    if let Some(timers) = &pm.timers {
        for t in timers {
            let t_kebab = t.name.to_case(Case::Kebab);
            let t_pascal = format_ident!("{}", t.name.to_case(Case::Pascal));
            let spec_ts = if let Some(after) = &t.after {
                let unit_ident = match after.unit.to_lowercase().as_str() {
                    "minute" | "minutes" => format_ident!("minutes"),
                    "hour" | "hours" => format_ident!("hours"),
                    "day" | "days" => format_ident!("days"),
                    "week" | "weeks" => format_ident!("weeks"),
                    "month" | "months" => format_ident!("months"),
                    "year" | "years" => format_ident!("years"),
                    _ => format_ident!("seconds"),
                };
                let val_lit = proc_macro2::Literal::u64_unsuffixed(after.value.max(1) as u64);
                quote! { #t_pascal(after(#unit_ident = #val_lit)) }
            } else if let Some(at_field) = &t.at {
                let field_ident = format_ident!("{}", at_field.to_case(Case::Snake));
                quote! { #t_pascal(at = #field_ident) }
            } else {
                quote! { #t_pascal(after(minutes = 1)) }
            };
            declared_timers.insert(t_kebab, (t_pascal, spec_ts));
        }
    }

    for t_kebab in referenced_timer_kebabs {
        declared_timers.entry(t_kebab.clone()).or_insert_with(|| {
            let t_pascal = format_ident!("{}", t_kebab.to_case(Case::Pascal));
            let spec_ts = quote! { #t_pascal(after(minutes = 1)) };
            (t_pascal, spec_ts)
        });
    }

    let timer_specs: Vec<(syn::Ident, proc_macro2::TokenStream)> =
        declared_timers.values().cloned().collect();

    let mut merged_timer_effects_by_event: HashMap<syn::Ident, (Vec<syn::Ident>, Vec<syn::Ident>)> =
        HashMap::new();
    for (key, (sets, cancels)) in raw_timer_effects_by_event {
        if let Some(ev_ident) = event_ident_by_key.get(&key) {
            let set_idents = sets
                .iter()
                .map(|k| format_ident!("{}", k.to_case(Case::Pascal)))
                .collect();
            let cancel_idents = cancels
                .iter()
                .map(|k| format_ident!("{}", k.to_case(Case::Pascal)))
                .collect();
            merged_timer_effects_by_event.insert(ev_ident.clone(), (set_idents, cancel_idents));
        }
    }

    let mut merged_timer_effects_by_timer: HashMap<syn::Ident, (Vec<syn::Ident>, Vec<syn::Ident>)> =
        HashMap::new();
    for (t_kebab, (sets, cancels)) in raw_timer_effects_by_timer {
        let t_ident = format_ident!("{}", t_kebab.to_case(Case::Pascal));
        let set_idents = sets
            .iter()
            .map(|k| format_ident!("{}", k.to_case(Case::Pascal)))
            .collect();
        let cancel_idents = cancels
            .iter()
            .map(|k| format_ident!("{}", k.to_case(Case::Pascal)))
            .collect();
        merged_timer_effects_by_timer.insert(t_ident, (set_idents, cancel_idents));
    }

    let has_ends_when = pm.ends_when.as_ref().is_some_and(|v| !v.is_empty());

    ResolvedPmSpec {
        event_imports,
        command_imports,
        starts_when_idents,
        handles_idents,
        emit_cmd_idents,
        timer_specs,
        has_ends_when,
        event_ident_by_key,
        event_ident_by_event_name,
        merged_timer_effects_by_event,
        merged_timer_effects_by_timer,
    }
}

pub fn generate_process_managers(
    out_dir: &PathBuf,
    process_managers: &[EsdmDoc],
    routing: &crate::common::RoutingContext,
) -> Result<()> {
    if process_managers.is_empty() {
        return Ok(());
    }

    let pm_base_dir = out_dir.join("integration/process-managers");
    fs::create_dir_all(&pm_base_dir)?;

    for pm in process_managers {
        let pm_name = &pm.name;
        let pm_pascal = pm_name.to_case(Case::Pascal);
        let pm_ident = format_ident!("{}", pm_pascal);
        let pm_commands_ident = format_ident!("{}Commands", pm_pascal);
        let domain_pascal =
            routing.domain_ident(pm.scope.as_ref().and_then(|s| s.domain.as_deref()));

        let pm_dir = pm_base_dir.join(pm_name);
        fs::create_dir_all(&pm_dir)?;

        let mut dep_bcs = HashSet::new();
        if let Some(starts_when) = &pm.starts_when {
            for sw in starts_when {
                if let Some(bc) = &sw.bounded_context {
                    dep_bcs.insert(bc.clone());
                }
            }
        }
        if let Some(reactions) = &pm.reactions {
            for r in reactions {
                if let Some(when) = &r.when
                    && let Some(bc) = &when.bounded_context
                {
                    dep_bcs.insert(bc.clone());
                }
                if let Some(emits) = &r.emits {
                    for e in emits {
                        if let Some(bc) = &e.bounded_context {
                            dep_bcs.insert(bc.clone());
                        }
                    }
                }
            }
        }

        let domain_rel_path =
            crate::common::get_relative_cargo_path(&pm_dir, &out_dir.join("domain"));

        let mut cargo_toml = format!(
            r#"[package]
name = "{pm_name}"
version = "0.1.0"
edition = "2024"

[dependencies]
chromatofoor = {{ workspace = true }}
domain = {{ path = "{domain_rel_path}" }}
anyhow = {{ workspace = true }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ workspace = true }}
uuid = {{ version = "1.11", features = ["serde", "v4"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
thiserror = "2.0"
tracing = {{ workspace = true }}
"#
        );

        let mut sorted_dep_bcs: Vec<_> = dep_bcs.iter().collect();
        sorted_dep_bcs.sort();
        for bc in sorted_dep_bcs {
            let bc_dir = crate::common::resolve_bc_dir(out_dir, bc, routing);
            let rel_path = pathdiff::diff_paths(&bc_dir, &pm_dir).unwrap();
            let rel_path_str = rel_path.to_str().unwrap().replace('\\', "/");
            cargo_toml.push_str(&format!("{bc} = {{ path = \"{rel_path_str}\" }}\n"));
        }

        cargo_toml.push_str(
            r#"
[dev-dependencies]
tokio = { workspace = true }
rstest = "0.23"
serde_json = { workspace = true }
tracing-test = { workspace = true }
"#,
        );
        cargo_toml.push_str(crate::common::MEMBER_LINTS_TOML);

        fs::write(pm_dir.join("Cargo.toml"), cargo_toml)?;

        let src_dir = pm_dir.join("src");
        fs::create_dir_all(&src_dir)?;
        let gen_file = src_dir.join("generated.rs");
        let manual_file = src_dir.join("process_manager.rs");

        let mut state_fields = Vec::new();
        let mut extra_structs = Vec::new();

        if let Some(state) = &pm.state {
            for (prop_name, prop_schema) in &state.properties {
                let prop_snake = prop_name.to_case(Case::Snake);
                let prop_ident = format_ident!("{}", prop_snake);
                let (prop_ts, generated_extras) =
                    crate::common::generate_type_for_prop(prop_name, prop_schema, &pm_pascal);
                extra_structs.extend(generated_extras);

                let serde_attr = if prop_snake == *prop_name {
                    quote! { #[serde(default)] }
                } else {
                    quote! { #[serde(default, rename = #prop_name, alias = #prop_snake)] }
                };

                state_fields.push(quote! {
                    #serde_attr
                    pub #prop_ident: Option<#prop_ts>
                });
            }
        }

        let spec = resolve_pm_spec(pm);

        let import_tokens: Vec<proc_macro2::TokenStream> = spec
            .event_imports
            .iter()
            .chain(spec.command_imports.iter())
            .map(|s| syn::parse_str(s).unwrap())
            .collect();

        let corr_tokens = pm
            .correlated_by
            .as_ref()
            .filter(|c| c.source == "event-field")
            .and_then(|c| c.field.as_deref())
            .map_or_else(
                || quote! { event_subject },
                |f| {
                    let f_ident = format_ident!("{}", f.to_case(Case::Snake));
                    quote! { event_field(#f_ident) }
                },
            );

        let starts_when_idents = &spec.starts_when_idents;
        let handles_attr = if spec.handles_idents.is_empty() {
            quote! {}
        } else {
            let handles_idents = &spec.handles_idents;
            quote! { handles = [#(#handles_idents),*], }
        };
        let emits_attr = if spec.emit_cmd_idents.is_empty() {
            quote! {}
        } else {
            let emit_cmd_idents = &spec.emit_cmd_idents;
            quote! { emits = [#(#emit_cmd_idents),*], }
        };
        let timers_attr = if spec.timer_specs.is_empty() {
            quote! {}
        } else {
            let timer_tokens: Vec<_> = spec.timer_specs.iter().map(|(_, ts)| ts).collect();
            quote! { timers = [#(#timer_tokens),*], }
        };
        let dg_and_idem = crate::common::format_delivery_and_idempotency_tokens(pm);
        let pm_doc = crate::common::make_doc_comment(pm.description.as_deref());

        let lib_tokens = quote! {
            pub mod generated;
            pub mod process_manager;

            pub use generated::*;
        };
        crate::common::write_rust_file(&src_dir.join("lib.rs"), lib_tokens)?;

        let pm_tokens = quote! {
            #[allow(unused_imports)]
            use chromatofoor::validator::Validate;
            #[allow(unused_imports)]
            use serde::{Deserialize, Serialize};
            #(#import_tokens)*

            #(#extra_structs)*

            #pm_doc
            #[chromatofoor::process_manager(
                scope = ::domain::#domain_pascal,
                correlated_by = #corr_tokens,
                starts_when = [#(#starts_when_idents),*],
                #handles_attr
                #emits_attr
                #timers_attr
                ends_when = Self::has_ended,
                #dg_and_idem
            )]
            #[derive(Debug, Clone, Default, PartialEq)]
            pub struct #pm_ident {
                #(#state_fields),*
            }
        };
        crate::common::write_rust_file(&gen_file, pm_tokens)?;

        let all_handled_events: Vec<syn::Ident> = spec
            .starts_when_idents
            .iter()
            .chain(spec.handles_idents.iter())
            .cloned()
            .collect();

        if manual_file.exists() {
            sync_manual_pm_impls(
                &manual_file,
                &pm_ident,
                &pm_commands_ident,
                &all_handled_events,
                &spec,
            )?;
        } else {
            let event_impls: Vec<_> = all_handled_events
                .iter()
                .map(|ev_ident| {
                    let body = build_pm_todo_expr(
                        &format!("implement OnEvent::{ev_ident}::on_event for {pm_ident}"),
                        spec.merged_timer_effects_by_event.get(ev_ident),
                    );
                    quote! {
                        impl OnEvent<#ev_ident> for #pm_ident {
                            fn on_event(
                                &mut self,
                                _event: Envelope<#ev_ident>,
                            ) -> Vec<#pm_commands_ident> {
                                #body
                            }
                        }
                    }
                })
                .collect();

            let timer_impls: Vec<_> = spec
                .timer_specs
                .iter()
                .map(|(timer_ident, _)| {
                    let body = build_pm_todo_expr(
                        &format!("implement OnTimer::{timer_ident}::on_timer for {pm_ident}"),
                        spec.merged_timer_effects_by_timer.get(timer_ident),
                    );
                    quote! {
                        impl OnTimer<#timer_ident> for #pm_ident {
                            fn on_timer(&mut self) -> Vec<#pm_commands_ident> {
                                #body
                            }
                        }
                    }
                })
                .collect();

            let integration_imports = if timer_impls.is_empty() {
                quote! {
                    use chromatofoor::{architecturekit::Envelope, integration::OnEvent};
                }
            } else {
                quote! {
                    use chromatofoor::{
                        architecturekit::Envelope,
                        integration::{OnEvent, OnTimer},
                    };
                }
            };

            let has_ended_body = build_pm_has_ended_expr(&pm_ident, spec.has_ends_when);

            let manual_tokens = quote! {
                use crate::generated::*;
                #integration_imports

                impl #pm_ident {
                    #[must_use]
                    pub fn has_ended(&self) -> bool {
                        #has_ended_body
                    }
                }

                #(#event_impls)*
                #(#timer_impls)*
            };
            crate::common::write_manual_rust_file(&manual_file, manual_tokens)?;
        }
    }

    Ok(())
}

fn build_pm_has_ended_expr(pm_ident: &syn::Ident, has_ends_when: bool) -> proc_macro2::TokenStream {
    if has_ends_when {
        let msg = format!("implement {pm_ident}::has_ended");
        quote! { todo!(#msg) }
    } else {
        quote! { false }
    }
}

fn build_pm_todo_expr(
    base: &str,
    effects: Option<&(Vec<syn::Ident>, Vec<syn::Ident>)>,
) -> proc_macro2::TokenStream {
    let msg = if let Some((sets, cancels)) = effects
        && (!sets.is_empty() || !cancels.is_empty())
    {
        let set_list = sets
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let cancel_list = cancels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{base} (declared timers: set=[{set_list}], cancel=[{cancel_list}])")
    } else {
        base.to_string()
    };
    quote! { todo!(#msg) }
}

fn sync_manual_pm_impls(
    manual_file: &std::path::Path,
    pm_ident: &syn::Ident,
    pm_commands_ident: &syn::Ident,
    all_handled_events: &[syn::Ident],
    spec: &ResolvedPmSpec,
) -> Result<()> {
    let existing_content = fs::read_to_string(manual_file)?;
    let Ok(syntax_tree) = syn::parse_file(&existing_content) else {
        return Ok(());
    };

    let target_name = pm_ident.to_string();
    let mut has_has_ended = false;
    let mut existing_on_events = HashSet::new();
    let mut existing_on_timers = HashSet::new();

    for item in &syntax_tree.items {
        if let syn::Item::Impl(item_impl) = item
            && let syn::Type::Path(self_path) = &*item_impl.self_ty
            && self_path
                .path
                .segments
                .last()
                .is_some_and(|seg| seg.ident == target_name)
        {
            if let Some((_, trait_path, _)) = &item_impl.trait_ {
                if let Some(last_seg) = trait_path.segments.last()
                    && let syn::PathArguments::AngleBracketed(args) = &last_seg.arguments
                    && let Some(syn::GenericArgument::Type(syn::Type::Path(arg_path))) =
                        args.args.first()
                    && let Some(arg_last) = arg_path.path.segments.last()
                {
                    let arg_name = syn::ext::IdentExt::unraw(&arg_last.ident).to_string();
                    if last_seg.ident == "OnEvent" {
                        existing_on_events.insert(arg_name);
                    } else if last_seg.ident == "OnTimer" {
                        existing_on_timers.insert(arg_name);
                    }
                }
            } else {
                for impl_item in &item_impl.items {
                    if let syn::ImplItem::Fn(method) = impl_item
                        && syn::ext::IdentExt::unraw(&method.sig.ident) == "has_ended"
                    {
                        has_has_ended = true;
                    }
                }
            }
        }
    }

    let mut missing_items = Vec::new();
    if !has_has_ended {
        let has_ended_body = build_pm_has_ended_expr(pm_ident, spec.has_ends_when);
        missing_items.push(quote! {
            impl #pm_ident {
                #[must_use]
                pub fn has_ended(&self) -> bool {
                    #has_ended_body
                }
            }
        });
    }

    for ev_ident in all_handled_events {
        let ev_name = syn::ext::IdentExt::unraw(ev_ident).to_string();
        if !existing_on_events.contains(&ev_name) {
            let body = build_pm_todo_expr(
                &format!("implement OnEvent::{ev_ident}::on_event for {pm_ident}"),
                spec.merged_timer_effects_by_event.get(ev_ident),
            );
            missing_items.push(quote! {
                impl chromatofoor::integration::OnEvent<#ev_ident> for #pm_ident {
                    fn on_event(
                        &mut self,
                        _event: chromatofoor::architecturekit::Envelope<#ev_ident>,
                    ) -> Vec<#pm_commands_ident> {
                        #body
                    }
                }
            });
        }
    }

    for (timer_ident, _) in &spec.timer_specs {
        let timer_name = syn::ext::IdentExt::unraw(timer_ident).to_string();
        if !existing_on_timers.contains(&timer_name) {
            let body = build_pm_todo_expr(
                &format!("implement OnTimer::{timer_ident}::on_timer for {pm_ident}"),
                spec.merged_timer_effects_by_timer.get(timer_ident),
            );
            missing_items.push(quote! {
                impl chromatofoor::integration::OnTimer<#timer_ident> for #pm_ident {
                    fn on_timer(&mut self) -> Vec<#pm_commands_ident> {
                        #body
                    }
                }
            });
        }
    }

    if missing_items.is_empty() {
        return Ok(());
    }

    let wrapper_file: syn::File = syn::parse2(quote! {
        #(#missing_items)*
    })?;
    let formatted_appended = prettyplease::unparse(&wrapper_file);

    let mut updated = existing_content;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push('\n');
    updated.push_str(&formatted_appended);
    fs::write(manual_file, updated)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::*;

    #[test]
    fn test_pm_disambiguates_collisions_merges_reactions_and_syncs_manual_methods() {
        let temp_dir = tempfile::tempdir().unwrap();
        let out_dir = temp_dir.path().to_path_buf();
        let routing = crate::common::RoutingContext {
            domain_name: "commerce".to_string(),
            ..Default::default()
        };

        let pm_v1 = EsdmDoc {
            name: "order-saga".to_string(),
            kind: "process-manager".to_string(),
            reactions: Some(vec![Reaction {
                when: Some(ReactionWhen {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    event: Some("created".to_string()),
                    timer: None,
                }),
                rule: None,
                emits: Some(vec![CommandRef {
                    bounded_context: Some("sales".to_string()),
                    aggregate: Some("order".to_string()),
                    command: Some("cancel".to_string()),
                }]),
                set_timers: Some(vec!["first-timer".to_string()]),
                cancel_timers: None,
            }]),
            ..Default::default()
        };

        generate_process_managers(&out_dir, &[pm_v1], &routing).unwrap();

        let manual_path =
            out_dir.join("integration/process-managers/order-saga/src/process_manager.rs");
        std::fs::write(
            &manual_path,
            "use crate::generated::*;\n\nimpl OrderSaga {\n    // preserve user comment with brace }\n    #[must_use]\n    pub fn has_ended(&self) -> bool {\n        true\n    }\n}\n\nfn trailing_helper() -> &'static str {\n    \"}\"\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn sample() {}\n}\n// trailing comment with }\n",
        )
        .unwrap();

        // Regenerate with colliding event/command names across BCs and a second reaction on the same event
        let pm_v2 = EsdmDoc {
            name: "order-saga".to_string(),
            kind: "process-manager".to_string(),
            reactions: Some(vec![
                Reaction {
                    when: Some(ReactionWhen {
                        bounded_context: Some("sales".to_string()),
                        aggregate: Some("order".to_string()),
                        event: Some("created".to_string()),
                        timer: None,
                    }),
                    rule: None,
                    emits: Some(vec![CommandRef {
                        bounded_context: Some("sales".to_string()),
                        aggregate: Some("order".to_string()),
                        command: Some("cancel".to_string()),
                    }]),
                    set_timers: Some(vec!["first-timer".to_string()]),
                    cancel_timers: None,
                },
                Reaction {
                    when: Some(ReactionWhen {
                        bounded_context: Some("sales".to_string()),
                        aggregate: Some("order".to_string()),
                        event: Some("created".to_string()),
                        timer: None,
                    }),
                    rule: None,
                    emits: Some(vec![CommandRef {
                        bounded_context: Some("billing".to_string()),
                        aggregate: Some("invoice".to_string()),
                        command: Some("cancel".to_string()),
                    }]),
                    set_timers: Some(vec!["second-timer".to_string()]),
                    cancel_timers: None,
                },
                Reaction {
                    when: Some(ReactionWhen {
                        bounded_context: Some("billing".to_string()),
                        aggregate: Some("invoice".to_string()),
                        event: Some("created".to_string()),
                        timer: None,
                    }),
                    rule: None,
                    emits: None,
                    set_timers: None,
                    cancel_timers: None,
                },
            ]),
            ..Default::default()
        };

        generate_process_managers(&out_dir, &[pm_v2], &routing).unwrap();

        let gen_rs = std::fs::read_to_string(
            out_dir.join("integration/process-managers/order-saga/src/generated.rs"),
        )
        .unwrap();
        // Both colliding commands and events are disambiguated in `#[chromatofoor::process_manager(...)]`
        assert!(gen_rs.contains("as SalesOrderCancel;"));
        assert!(gen_rs.contains("as BillingInvoiceCancel;"));
        assert!(gen_rs.contains("as SalesOrderCreated;"));
        assert!(gen_rs.contains("as BillingInvoiceCreated;"));
        assert!(gen_rs.contains("FirstTimer(after(minutes = 1))"));
        assert!(gen_rs.contains("SecondTimer(after(minutes = 1))"));
        assert!(gen_rs.contains("pub struct OrderSaga"));

        let pm_cargo = std::fs::read_to_string(
            out_dir.join("integration/process-managers/order-saga/Cargo.toml"),
        )
        .unwrap();
        assert!(pm_cargo.contains("[lints]\nworkspace = true"));

        let manual_rs = std::fs::read_to_string(&manual_path).unwrap();
        assert!(manual_rs.contains("// preserve user comment with brace }"));
        assert!(manual_rs.contains("fn trailing_helper()"));
        assert!(manual_rs.contains("// trailing comment with }"));
        // Merged timer effects embedded inside todo!(...) string literal
        assert!(manual_rs.contains(
            "implement OnEvent::SalesOrderCreated::on_event for OrderSaga (declared timers: set=[FirstTimer, SecondTimer], cancel=[])"
        ));
        assert!(manual_rs.contains(
            "implement OnEvent::BillingInvoiceCreated::on_event for OrderSaga"
        ));
        assert!(manual_rs.contains("implement OnTimer::FirstTimer::on_timer for OrderSaga"));
        assert!(manual_rs.contains("implement OnTimer::SecondTimer::on_timer for OrderSaga"));

        let parsed = syn::parse_file(&manual_rs).expect("manual process_manager.rs should parse");
        assert!(manual_rs.contains("OnEvent<SalesOrderCreated> for OrderSaga"));
        assert!(manual_rs.contains("OnEvent<BillingInvoiceCreated> for OrderSaga"));
        assert!(manual_rs.contains("OnTimer<FirstTimer> for OrderSaga"));
        assert!(manual_rs.contains("OnTimer<SecondTimer> for OrderSaga"));
        assert!(!parsed.items.is_empty());

        // Verify endsWhen produces todo!() in has_ended when present vs false when absent
        let pm_ends = EsdmDoc {
            name: "closing-saga".to_string(),
            kind: "process-manager".to_string(),
            ends_when: Some(vec![NamedCondition {
                name: "done".to_string(),
                condition: Some("completed".to_string()),
            }]),
            ..Default::default()
        };
        generate_process_managers(&out_dir, &[pm_ends], &routing).unwrap();
        let closing_manual = std::fs::read_to_string(
            out_dir.join("integration/process-managers/closing-saga/src/process_manager.rs"),
        )
        .unwrap();
        assert!(closing_manual.contains("implement ClosingSaga::has_ended"));
    }
}
