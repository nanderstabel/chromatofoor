use crate::generator::process_manager::{resolve_pm_event_bc, resolve_pm_spec};
use crate::model::EsdmDoc;
use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::fs;
use std::path::PathBuf;

pub fn generate_process_manager_tests(
    out_dir: &PathBuf,
    process_managers: &[EsdmDoc],
    all_docs: &[EsdmDoc],
) -> Result<()> {
    if process_managers.is_empty() {
        return Ok(());
    }

    for pm in process_managers {
        let pm_name = &pm.name;
        let pm_snake = format_ident!("{}", pm_name.to_case(Case::Snake));
        let pm_pascal = format_ident!("{}", pm_name.to_case(Case::Pascal));
        let pm_events_ident = format_ident!("{}Events", pm_pascal);
        let spec = resolve_pm_spec(pm);

        let test_file_name = format!("{}_tests", pm_name.to_case(Case::Snake));
        let pm_dir = out_dir.join("integration/process-managers").join(pm_name);
        let tests_dir = pm_dir.join("tests");
        fs::create_dir_all(&tests_dir)?;

        let resolve_ev_ident =
            |bc_opt: Option<&str>, agg_opt: Option<&str>, ev_name: &str| -> syn::Ident {
                if let Some(agg) = agg_opt {
                    let bc = resolve_pm_event_bc(pm, bc_opt, agg, ev_name);
                    if let Some(ident) = spec.event_ident_by_key.get(&(
                        bc.to_string(),
                        agg.to_string(),
                        ev_name.to_string(),
                    )) {
                        return ident.clone();
                    }
                }
                spec.event_ident_by_event_name
                    .get(ev_name)
                    .cloned()
                    .unwrap_or_else(|| format_ident!("{}", ev_name.to_case(Case::Pascal)))
            };

        let pm_features: Vec<&EsdmDoc> = all_docs
            .iter()
            .filter(|doc| {
                doc.kind == "feature"
                    && doc
                        .scope
                        .as_ref()
                        .and_then(|s| s.process_manager.as_deref())
                        == Some(pm_name.as_str())
            })
            .collect();

        let mut scenario_tests = Vec::new();
        for feature in pm_features {
            if let Some(scenarios) = &feature.scenarios {
                for scenario in scenarios {
                    let test_name = format_ident!(
                        "test_{}_{}",
                        feature.name.to_case(Case::Snake),
                        scenario.name.to_case(Case::Snake)
                    );

                    let mut given_stmts = Vec::new();
                    for g in &scenario.given {
                        let ev_ident = resolve_ev_ident(
                            g.bounded_context.as_deref(),
                            g.aggregate.as_deref(),
                            &g.event,
                        );
                        let json_str = serde_json::to_string(&g.data).unwrap_or_default();
                        given_stmts.push(quote! {
                            #pm_events_ident::from(
                                serde_json::from_str::<#pm_snake::#ev_ident>(#json_str)
                                    .expect("Failed to deserialize given event"),
                            )
                        });
                    }

                    let when_stmt = if let Some(ev_name) = &scenario.when.event {
                        let ev_ident = resolve_ev_ident(
                            scenario.when.bounded_context.as_deref(),
                            scenario.when.aggregate.as_deref(),
                            ev_name,
                        );
                        let json_str =
                            serde_json::to_string(&scenario.when.data).unwrap_or_default();
                        quote! {
                            .when_event(
                                serde_json::from_str::<#pm_snake::#ev_ident>(#json_str)
                                    .expect("Failed to deserialize when event"),
                            )
                        }
                    } else if let Some(timer_name) = &scenario.when.timer {
                        let timer_pascal = format_ident!("{}", timer_name.to_case(Case::Pascal));
                        quote! {
                            .when_timer::<#pm_snake::#timer_pascal>()
                        }
                    } else {
                        continue;
                    };

                    let mut then_stmts = Vec::new();
                    if let Some(emits) = &scenario.then.emits {
                        if emits.is_empty() {
                            then_stmts.push(quote! { .then_emits([]) });
                        } else {
                            for emit in emits {
                                let bc = &emit.bounded_context;
                                let agg = &emit.aggregate;
                                let cmd = &emit.command;
                                let json_str =
                                    serde_json::to_string(&emit.data).unwrap_or_default();
                                then_stmts.push(quote! {
                                    .then_emits_command(
                                        #bc,
                                        #agg,
                                        #cmd,
                                        Some(&serde_json::from_str(#json_str).expect("Failed to deserialize expected emitted command")),
                                    )
                                });
                            }
                        }
                    }
                    if let Some(commands) = &scenario.then.commands {
                        if commands.is_empty() {
                            then_stmts.push(quote! { .then_emits([]) });
                        } else {
                            for emit in commands {
                                let bc = &emit.bounded_context;
                                let agg = &emit.aggregate;
                                let cmd = &emit.command;
                                let json_str =
                                    serde_json::to_string(&emit.data).unwrap_or_default();
                                then_stmts.push(quote! {
                                    .then_emits_command(
                                        #bc,
                                        #agg,
                                        #cmd,
                                        Some(&serde_json::from_str(#json_str).expect("Failed to deserialize expected emitted command")),
                                    )
                                });
                            }
                        }
                    }
                    if let Some(set_timers) = &scenario.then.set_timers {
                        then_stmts.push(quote! {
                            .then_set_timers([#(#set_timers),*])
                        });
                    }
                    if let Some(cancel_timers) = &scenario.then.cancel_timers {
                        then_stmts.push(quote! {
                            .then_cancel_timers([#(#cancel_timers),*])
                        });
                    }
                    if let Some(expected_state) = &scenario.then.state {
                        let json_str = serde_json::to_string(expected_state).unwrap_or_default();
                        then_stmts.push(quote! {
                            .then_state_json(
                                &serde_json::from_str(#json_str).expect("Failed to deserialize expected state"),
                            )
                        });
                    }
                    if scenario.then.ended == Some(true) {
                        then_stmts.push(quote! {
                            .then_ended()
                        });
                    }

                    scenario_tests.push(quote! {
                        #[tokio::test]
                        async fn #test_name() {
                            chromatofoor::testing::given_process_manager::<#pm_pascal>(vec![
                                #(#given_stmts),*
                            ])
                            #when_stmt
                            #(#then_stmts)*;
                        }
                    });
                }
            }
        }

        let test_tokens = quote! {
            #![allow(unused_imports)]
            use #pm_snake::{#pm_events_ident, #pm_pascal};

            #[tokio::test]
            async fn test_process_manager_state_init() {
                let state = #pm_pascal::default();
                let json = serde_json::to_value(&state).unwrap();
                assert!(json.is_object());
            }

            #(#scenario_tests)*
        };

        crate::generator::write_rust_file(
            &tests_dir.join(format!("{test_file_name}.rs")),
            test_tokens,
        )?;
    }
    Ok(())
}
