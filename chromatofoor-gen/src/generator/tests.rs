use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::generator::GeneratorContext;

fn resolve_dcb_event_owners(
    bc: &str,
    agg: &str,
    ctx: &GeneratorContext,
    contexts: &HashMap<(String, String), GeneratorContext>,
) -> Vec<(String, String)> {
    let mut dcb_events: Vec<(String, String)> = Vec::new();
    let mut seen_dcb_events = HashSet::new();
    for ev in &ctx.events {
        if seen_dcb_events.insert((agg.to_string(), ev.name.clone())) {
            dcb_events.push((agg.to_string(), ev.name.clone()));
        }
    }
    if let Some(boundary) = &ctx.boundary
        && let Some(consults) = &boundary.consults
    {
        let mut sorted_keys: Vec<_> = contexts.keys().collect();
        sorted_keys.sort();
        for consult in consults {
            if consult
                .bounded_context
                .as_deref()
                .is_some_and(|c_bc| c_bc != bc)
            {
                continue;
            }
            let Some(ev_name) = &consult.event else {
                continue;
            };
            if let Some(target_agg) = &consult.aggregate {
                if let Some(target_ctx) = contexts.get(&(bc.to_string(), target_agg.clone()))
                    && target_ctx.events.iter().any(|e| &e.name == ev_name)
                    && seen_dcb_events.insert((target_agg.clone(), ev_name.clone()))
                {
                    dcb_events.push((target_agg.clone(), ev_name.clone()));
                }
            } else {
                for (c_bc, c_unit) in &sorted_keys {
                    if c_bc == bc
                        && c_unit != "global"
                        && let Some(target_ctx) =
                            contexts.get(&((*c_bc).clone(), (*c_unit).clone()))
                        && target_ctx.events.iter().any(|e| &e.name == ev_name)
                    {
                        if seen_dcb_events.insert(((*c_unit).clone(), ev_name.clone())) {
                            dcb_events.push(((*c_unit).clone(), ev_name.clone()));
                        }
                        break;
                    }
                }
            }
        }
    }
    dcb_events
}

pub fn generate_tests(
    out_dir: &PathBuf,
    contexts: &HashMap<(String, String), GeneratorContext>,
    bc_contexts: &HashMap<String, crate::generator::BoundedContextGenerator>,
    event_to_owner: &HashMap<String, (String, String)>,
    routing: &crate::generator::RoutingContext,
) -> Result<()> {
    // Group features by bounded context
    let mut bc_features = std::collections::BTreeMap::new();

    let mut sorted_contexts: Vec<_> = contexts.iter().collect();
    sorted_contexts.sort_by_key(|((bc, agg), _)| (bc.clone(), agg.clone()));

    for ((bc, agg), ctx) in sorted_contexts {
        if bc == "global" || agg == "global" {
            continue;
        }

        if !ctx.features.is_empty() && (ctx.aggregate.is_some() || ctx.boundary.is_some()) {
            let dcb_events = if ctx.boundary.is_some() {
                resolve_dcb_event_owners(bc, agg, ctx, contexts)
            } else {
                Vec::new()
            };
            bc_features
                .entry(bc.clone())
                .or_insert_with(Vec::new)
                .push((
                    agg.clone(),
                    &ctx.features,
                    ctx.boundary.is_some(),
                    dcb_events,
                ));
        }
    }

    for (bc, features_by_agg) in bc_features {
        let bc_dir = crate::generator::resolve_bc_dir(out_dir, &bc, routing);
        let tests_dir = bc_dir.join("tests");
        fs::create_dir_all(&tests_dir)?;

        for (agg, features, is_boundary, dcb_events) in features_by_agg {
            let agg_snake = agg.to_case(Case::Snake);
            let test_file_name = format!("{agg_snake}_tests");

            let mut test_file_chunks = Vec::new();

            let agg_pascal = format_ident!("{}", agg.to_case(Case::Pascal));
            let agg_ident_snake = format_ident!("{}", agg_snake);
            let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));

            let error_ident = format_ident!("{}Error", agg_pascal);
            let command_enum = format_ident!("{}Command", agg_pascal);
            let event_enum = format_ident!("{}Event", agg_pascal);

            test_file_chunks.push(quote! { #![allow(unused_imports)] });
            test_file_chunks.push(quote! {
                use tracing_test::traced_test;

                use #bc_ident::generated::domain::#agg_ident_snake::aggregate::#agg_pascal;
                use #bc_ident::generated::domain::#agg_ident_snake::command::*;
                use #bc_ident::generated::domain::#agg_ident_snake::event::*;
                use #bc_ident::generated::domain::#agg_ident_snake::error::#error_ident;
            });

            for feature in features {
                if let Some(scenarios) = &feature.scenarios {
                    for scenario in scenarios {
                        let test_name = format_ident!("{}", scenario.name.to_case(Case::Snake));

                        let mut given_stmts = Vec::new();
                        for g in &scenario.given {
                            let ev_pascal = format_ident!("{}", g.event.to_case(Case::Pascal));
                            let json_str =
                                serde_json::to_string(&g.data).unwrap_or_else(|_| "{}".to_string());

                            if is_boundary {
                                let matched_owner = if let Some(explicit_agg) = &g.aggregate {
                                    dcb_events
                                        .iter()
                                        .find(|(owner, ev_name)| {
                                            owner == explicit_agg && ev_name == &g.event
                                        })
                                        .map(|(owner, _)| owner.as_str())
                                } else {
                                    dcb_events
                                        .iter()
                                        .find(|(_, ev_name)| ev_name == &g.event)
                                        .map(|(owner, _)| owner.as_str())
                                };
                                let Some(owner_unit) = matched_owner else {
                                    continue;
                                };
                                let owner_snake =
                                    format_ident!("{}", owner_unit.to_case(Case::Snake));
                                let ev_mod = format_ident!("{}", g.event.to_case(Case::Snake));
                                let expect_msg = format!(
                                    "Failed to deserialize payload for given event '{event_enum}::{ev_pascal}'"
                                );
                                given_stmts.push(quote! {
                                    #event_enum::from(
                                        serde_json::from_str::<#bc_ident::generated::domain::#owner_snake::event::#ev_mod::#ev_pascal>(#json_str)
                                            .expect(#expect_msg),
                                    )
                                });
                            } else {
                                let expect_msg = format!(
                                    "Failed to deserialize payload for given event '{event_enum}::{ev_pascal}'"
                                );
                                given_stmts.push(quote! {
                                    #event_enum::#ev_pascal(serde_json::from_str(#json_str).expect(#expect_msg))
                                });
                            }
                        }

                        let when_cmd = if let Some(cmd_name) = &scenario.when.command {
                            let cmd_pascal = format_ident!("{}", cmd_name.to_case(Case::Pascal));
                            let json_str = serde_json::to_string(&scenario.when.data)
                                .unwrap_or_else(|_| "{}".to_string());
                            let expect_msg = format!(
                                "Failed to deserialize payload for when command '{command_enum}::{cmd_pascal}'"
                            );
                            quote! {
                                #command_enum::#cmd_pascal(serde_json::from_str(#json_str).expect(#expect_msg))
                            }
                        } else {
                            quote! { panic!("No command specified in scenario") }
                        };

                        let when_call = if let Some(actor) = &scenario.when.actor {
                            quote! { .when_by(#actor, #when_cmd) }
                        } else {
                            quote! { .when(#when_cmd) }
                        };

                        let then_stmt = if let Some(expected_events) = &scenario.then.events {
                            if expected_events.is_empty() {
                                quote! { .then_nothing() }
                            } else {
                                let mut then_events = Vec::new();
                                for e in expected_events {
                                    let ev_pascal =
                                        format_ident!("{}", e.event.to_case(Case::Pascal));
                                    let json_str = serde_json::to_string(&e.data)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    let expect_msg = format!(
                                        "Failed to deserialize payload for then event '{event_enum}::{ev_pascal}'"
                                    );
                                    if is_boundary {
                                        let owner_unit = dcb_events
                                            .iter()
                                            .find(|(_, ev_name)| ev_name == &e.event)
                                            .map_or(agg.as_str(), |(owner, _)| owner.as_str());
                                        let owner_snake =
                                            format_ident!("{}", owner_unit.to_case(Case::Snake));
                                        let ev_mod =
                                            format_ident!("{}", e.event.to_case(Case::Snake));
                                        then_events.push(quote! {
                                            #event_enum::from(
                                                serde_json::from_str::<#bc_ident::generated::domain::#owner_snake::event::#ev_mod::#ev_pascal>(#json_str)
                                                    .expect(#expect_msg),
                                            )
                                        });
                                    } else {
                                        then_events.push(quote! {
                                            #event_enum::#ev_pascal(serde_json::from_str(#json_str).expect(#expect_msg))
                                        });
                                    }
                                }
                                quote! {
                                    .then_events(vec![
                                        #(#then_events),*
                                    ])
                                }
                            }
                        } else if let Some(rejection) = &scenario.then.rejection {
                            if let Some(inv) = &rejection.invariant {
                                let inv_pascal = format_ident!("{}", inv.to_case(Case::Pascal));
                                quote! {
                                    .then_rejected(#error_ident::#inv_pascal)
                                }
                            } else if let Some(reason) = &rejection.reason {
                                quote! {
                                    .then_rejected_reason(#reason)
                                }
                            } else {
                                quote! {
                                    .then_rejected_reason("Unknown rejection")
                                }
                            }
                        } else {
                            quote! {}
                        };

                        test_file_chunks.push(quote! {
                            #[tokio::test]
                            #[traced_test]
                            async fn #test_name() {
                                chromatofoor::testing::given::<#agg_pascal>(vec![
                                    #(#given_stmts),*
                                ])
                                #when_call
                                .await
                                #then_stmt;
                            }
                        });
                    }
                }
            }

            let final_tokens = quote! { #(#test_file_chunks)* };
            crate::generator::write_rust_file(
                &tests_dir.join(format!("{test_file_name}.rs")),
                final_tokens,
            )?;
        }
    }

    // Generate tests for Read Models
    let mut sorted_bc_contexts: Vec<_> = bc_contexts.iter().collect();
    sorted_bc_contexts.sort_by_key(|(bc, _)| *bc);

    let mut sorted_ctx_keys: Vec<_> = contexts.keys().collect();
    sorted_ctx_keys.sort();

    for (bc, bc_ctx) in sorted_bc_contexts {
        if bc == "global" {
            continue;
        }
        let bc_dir = crate::generator::resolve_bc_dir(out_dir, bc, routing);
        let tests_dir = bc_dir.join("tests");
        fs::create_dir_all(&tests_dir)?;

        let mut rm_features: std::collections::BTreeMap<String, Vec<&crate::model::EsdmDoc>> =
            std::collections::BTreeMap::new();
        for feature in &bc_ctx.features {
            if let Some(s) = &feature.scope
                && let Some(rm) = &s.read_model
            {
                rm_features.entry(rm.clone()).or_default().push(feature);
            }
        }

        for (rm, features) in rm_features {
            let rm_snake = format_ident!("{}", rm.to_case(Case::Snake));
            let rm_pascal = rm.to_case(Case::Pascal);
            let test_file_name = format!("{}_tests", rm.to_case(Case::Snake));
            let mut test_file_chunks = Vec::new();
            let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
            let rm_ident = format_ident!("{}", rm_pascal);
            let rm_events_ident = format_ident!("{}Events", rm_pascal);

            let mut projected_events: Vec<(String, String)> = Vec::new();
            let mut seen_projected = HashSet::new();
            if let Some(rm_doc) = bc_ctx.read_models.iter().find(|d| d.name == rm)
                && let Some(projections) = &rm_doc.projections
            {
                for proj in projections {
                    if proj
                        .bounded_context
                        .as_deref()
                        .is_some_and(|p_bc| p_bc != bc)
                    {
                        continue;
                    }
                    let Some(ev_name) = &proj.event else {
                        continue;
                    };
                    if let Some(agg) = &proj.aggregate {
                        if agg != "Unknown"
                            && (contexts.is_empty()
                                || contexts.get(&(bc.clone(), agg.clone())).is_some_and(|ctx| {
                                    ctx.events.iter().any(|e| &e.name == ev_name)
                                }))
                            && seen_projected.insert((agg.clone(), ev_name.clone()))
                        {
                            projected_events.push((agg.clone(), ev_name.clone()));
                        }
                    } else {
                        for (c_bc, c_unit) in &sorted_ctx_keys {
                            if c_bc == bc
                                && *c_unit != "global"
                                && let Some(ctx) =
                                    contexts.get(&((*c_bc).clone(), (*c_unit).clone()))
                                && ctx.events.iter().any(|e| &e.name == ev_name)
                            {
                                if seen_projected.insert(((*c_unit).clone(), ev_name.clone())) {
                                    projected_events.push(((*c_unit).clone(), ev_name.clone()));
                                }
                                break;
                            }
                        }
                    }
                }
            }

            test_file_chunks.push(quote! { #![allow(unused_imports)] });
            test_file_chunks.push(quote! {
                use #bc_ident::generated::application::read_models::#rm_snake::{#rm_events_ident, #rm_ident};
            });

            for feature in features {
                if let Some(scenarios) = &feature.scenarios {
                    for scenario in scenarios {
                        let test_name = format_ident!("{}", scenario.name.to_case(Case::Snake));

                        let mut given_events = Vec::new();
                        for g in &scenario.given {
                            let resolved_agg = if !projected_events.is_empty() {
                                if let Some(explicit_agg) = &g.aggregate {
                                    projected_events
                                        .iter()
                                        .find(|(owner, ev_name)| {
                                            owner == explicit_agg && ev_name == &g.event
                                        })
                                        .map(|(owner, _)| owner.clone())
                                } else {
                                    projected_events
                                        .iter()
                                        .find(|(_, ev_name)| ev_name == &g.event)
                                        .map(|(owner, _)| owner.clone())
                                }
                            } else if let Some(a) = &g.aggregate {
                                Some(a.clone())
                            } else {
                                event_to_owner.get(&g.event).map(|(_, a)| a.clone())
                            };

                            let Some(event_agg) = resolved_agg else {
                                continue;
                            };

                            let agg_ident = format_ident!("{}", event_agg.to_case(Case::Snake));
                            let ev_mod = format_ident!("{}", g.event.to_case(Case::Snake));
                            let ev_pascal = format_ident!("{}", g.event.to_case(Case::Pascal));
                            let json_str =
                                serde_json::to_string(&g.data).unwrap_or_else(|_| "{}".to_string());

                            given_events.push(quote! {
                                #rm_events_ident::from(
                                    serde_json::from_str::<#bc_ident::generated::domain::#agg_ident::event::#ev_mod::#ev_pascal>(#json_str)
                                        .expect("Failed to deserialize given event payload"),
                                )
                            });
                        }

                        let when_stmt = if let Some(query_name) = &scenario.when.query {
                            let q_snake = format_ident!("{}", query_name.to_case(Case::Snake));
                            let q_pascal = format_ident!("{}", query_name.to_case(Case::Pascal));
                            let params_json = serde_json::to_string(&scenario.when.parameters)
                                .unwrap_or_else(|_| "{}".to_string());
                            let query_expr = quote! {
                                serde_json::from_str::<#bc_ident::generated::application::query::#q_snake::#q_pascal>(#params_json)
                                    .expect("Failed to deserialize query parameters")
                            };
                            if let Some(actor) = &scenario.when.actor {
                                quote! {
                                    .when_by(#actor, #query_expr)
                                    .await
                                }
                            } else {
                                quote! {
                                    .when(#query_expr)
                                    .await
                                }
                            }
                        } else {
                            quote! {}
                        };

                        let then_stmt = if let Some(expected_val) = &scenario.then.read_model {
                            let expected_json_str = serde_json::to_string(expected_val)
                                .unwrap_or_else(|_| "{}".to_string());
                            quote! {
                                .then_read_model_json(
                                    &serde_json::from_str(#expected_json_str).expect("Failed to parse expected JSON"),
                                )
                            }
                        } else {
                            quote! {}
                        };

                        test_file_chunks.push(quote! {
                            #[tokio::test]
                            async fn #test_name() {
                                chromatofoor::testing::given_read_model(
                                    #rm_ident::default(),
                                    vec![
                                        #(#given_events),*
                                    ],
                                )
                                .await
                                #when_stmt
                                #then_stmt;
                            }
                        });
                    }
                }
            }

            let final_tokens = quote! { #(#test_file_chunks)* };
            let test_file_path = tests_dir.join(format!("{test_file_name}.rs"));
            crate::generator::write_rust_file(&test_file_path, final_tokens)?;
        }
    }

    Ok(())
}
