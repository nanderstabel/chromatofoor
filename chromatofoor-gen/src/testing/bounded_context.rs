use crate::common::{BoundedContextGenerator, GeneratorContext, RoutingContext, resolve_bc_dir};
use crate::testing::{
    sanitize_test_ident_segment, scenario_doc_comment, sync_generated_test_dir,
};
use anyhow::Result;
use convert_case::{Case, Casing};
use esdm::EsdmDoc;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

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
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    event_to_owner: &HashMap<String, (String, String)>,
    routing: &RoutingContext,
) -> Result<()> {
    let mut all_bcs: BTreeSet<String> = bc_contexts.keys().cloned().collect();
    for (bc, _) in contexts.keys() {
        all_bcs.insert(bc.clone());
    }
    all_bcs.remove("global");

    let mut sorted_ctx_keys: Vec<_> = contexts.keys().collect();
    sorted_ctx_keys.sort();

    for bc in all_bcs {
        let bc_dir = resolve_bc_dir(out_dir, &bc, routing);
        let bc_ident = format_ident!("{}", bc.to_case(Case::Snake));
        let mut modules: Vec<(String, TokenStream)> = Vec::new();

        // 1. Consistency unit scenarios (Aggregate & DCB)
        let mut bc_units: Vec<(&String, &GeneratorContext)> = contexts
            .iter()
            .filter_map(|((c_bc, agg), ctx)| {
                if c_bc == &bc && agg != "global" {
                    Some((agg, ctx))
                } else {
                    None
                }
            })
            .collect();
        bc_units.sort_by_key(|(agg, _)| *agg);

        for (agg, ctx) in bc_units {
            if ctx.features.is_empty() || (ctx.aggregate.is_none() && ctx.boundary.is_none()) {
                continue;
            }
            let is_boundary = ctx.boundary.is_some();
            let dcb_events = if is_boundary {
                resolve_dcb_event_owners(&bc, agg, ctx, contexts)
            } else {
                Vec::new()
            };

            let agg_snake = agg.to_case(Case::Snake);
            let module_prefix = if is_boundary { "dcb" } else { "aggregate" };
            let module_name = format!("{module_prefix}_{agg_snake}");

            let agg_pascal = format_ident!("{}", agg.to_case(Case::Pascal));
            let agg_ident_snake = format_ident!("{}", agg_snake);
            let error_ident = format_ident!("{}Error", agg_pascal);
            let command_enum = format_ident!("{}Command", agg_pascal);
            let event_enum = format_ident!("{}Event", agg_pascal);

            let mut scenario_fns = Vec::new();

            for feature in &ctx.features {
                let feature_seg = sanitize_test_ident_segment(&feature.name);
                if let Some(scenarios) = &feature.scenarios {
                    for scenario in scenarios {
                        let scenario_seg = sanitize_test_ident_segment(&scenario.name);
                        let test_name = format_ident!("test_{feature_seg}_{scenario_seg}");

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
                                        let ev_mod = format_ident!("{}", e.event.to_case(Case::Snake));
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

                        let scenario_doc = scenario_doc_comment(feature, scenario);
                        scenario_fns.push(quote! {
                            #scenario_doc
                            #[tokio::test]
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

            if !scenario_fns.is_empty() {
                let unit_tokens = quote! {
                    use #bc_ident::generated::domain::#agg_ident_snake::aggregate::#agg_pascal;
                    use #bc_ident::generated::domain::#agg_ident_snake::command::*;
                    use #bc_ident::generated::domain::#agg_ident_snake::error::#error_ident;
                    use #bc_ident::generated::domain::#agg_ident_snake::event::*;

                    #(#scenario_fns)*
                };
                modules.push((module_name, unit_tokens));
            }
        }

        // 2. Read Model scenarios
        if let Some(bc_ctx) = bc_contexts.get(&bc) {
            let mut rm_features: BTreeMap<String, Vec<&EsdmDoc>> = BTreeMap::new();
            for feature in &bc_ctx.features {
                if let Some(s) = &feature.scope
                    && let Some(rm) = &s.read_model
                {
                    rm_features.entry(rm.clone()).or_default().push(feature);
                }
            }

            for (rm, features) in rm_features {
                let rm_snake_str = rm.to_case(Case::Snake);
                let rm_snake = format_ident!("{}", rm_snake_str);
                let rm_pascal = rm.to_case(Case::Pascal);
                let module_name = format!("read_model_{rm_snake_str}");
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
                                    || contexts.get(&(bc.clone(), agg.clone())).is_some_and(
                                        |ctx| ctx.events.iter().any(|e| &e.name == ev_name),
                                    ))
                                && seen_projected.insert((agg.clone(), ev_name.clone()))
                            {
                                projected_events.push((agg.clone(), ev_name.clone()));
                            }
                        } else {
                            for (c_bc, c_unit) in &sorted_ctx_keys {
                                if *c_bc == bc
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

                let mut scenario_fns = Vec::new();

                for feature in features {
                    let feature_seg = sanitize_test_ident_segment(&feature.name);
                    if let Some(scenarios) = &feature.scenarios {
                        for scenario in scenarios {
                            let scenario_seg = sanitize_test_ident_segment(&scenario.name);
                            let test_name = format_ident!("test_{feature_seg}_{scenario_seg}");

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
                                let json_str = serde_json::to_string(&g.data)
                                    .unwrap_or_else(|_| "{}".to_string());

                                given_events.push(quote! {
                                    #rm_events_ident::from(
                                        serde_json::from_str::<#bc_ident::generated::domain::#agg_ident::event::#ev_mod::#ev_pascal>(#json_str)
                                            .expect("Failed to deserialize given event payload"),
                                    )
                                });
                            }

                            let has_query = scenario.when.query.is_some();
                            let when_stmt = if let Some(query_name) = &scenario.when.query {
                                let q_snake = format_ident!("{}", query_name.to_case(Case::Snake));
                                let q_pascal =
                                    format_ident!("{}", query_name.to_case(Case::Pascal));
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

                            let mut then_chains = Vec::new();
                            if has_query {
                                if let Some(rejection) = &scenario.then.rejection {
                                    let reason = rejection
                                        .reason
                                        .as_deref()
                                        .or(rejection.invariant.as_deref())
                                        .unwrap_or("Unknown rejection");
                                    then_chains.push(quote! {
                                        .then_rejected_reason(#reason)
                                    });
                                } else {
                                    if let Some(expected_res) = &scenario.then.result {
                                        let expected_json_str =
                                            serde_json::to_string(expected_res)
                                                .unwrap_or_else(|_| "{}".to_string());
                                        then_chains.push(quote! {
                                            .then_result_json(
                                                &serde_json::from_str(#expected_json_str)
                                                    .expect("Failed to parse expected query result JSON"),
                                            )
                                        });
                                    }
                                    if let Some(expected_rm) = &scenario.then.read_model {
                                        let expected_json_str =
                                            serde_json::to_string(expected_rm)
                                                .unwrap_or_else(|_| "{}".to_string());
                                        then_chains.push(quote! {
                                            .then_read_model_json(
                                                &serde_json::from_str(#expected_json_str)
                                                    .expect("Failed to parse expected read model JSON"),
                                            )
                                        });
                                    }
                                }
                            } else if let Some(expected_rm) = &scenario.then.read_model {
                                let expected_json_str = serde_json::to_string(expected_rm)
                                    .unwrap_or_else(|_| "{}".to_string());
                                then_chains.push(quote! {
                                    .then_read_model_json(
                                        &serde_json::from_str(#expected_json_str)
                                            .expect("Failed to parse expected read model JSON"),
                                    )
                                });
                            }

                            let scenario_doc = scenario_doc_comment(feature, scenario);
                            scenario_fns.push(quote! {
                                #scenario_doc
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
                                    #(#then_chains)*;
                                }
                            });
                        }
                    }
                }

                if !scenario_fns.is_empty() {
                    let rm_tokens = quote! {
                        use #bc_ident::generated::application::read_models::#rm_snake::{#rm_events_ident, #rm_ident};

                        #(#scenario_fns)*
                    };
                    modules.push((module_name, rm_tokens));
                }
            }
        }

        sync_generated_test_dir(&bc_dir, &modules)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use esdm::*;

    #[test]
    fn test_read_model_query_result_and_rejection_assertions() {
        let rm_doc = EsdmDoc {
            name: "account-summary".to_string(),
            kind: "read-model".to_string(),
            ..Default::default()
        };
        let feature_doc = EsdmDoc {
            name: "query-account-summary".to_string(),
            kind: "feature".to_string(),
            scope: Some(Scope {
                bounded_context: Some("finance".to_string()),
                read_model: Some("account-summary".to_string()),
                ..Default::default()
            }),
            scenarios: Some(vec![
                Scenario {
                    name: "returns projected balance".to_string(),
                    description: Some("Projects balance updates after deposit.".to_string()),
                    given: vec![],
                    when: WhenAction {
                        event: None,
                        bounded_context: None,
                        aggregate: None,
                        timer: None,
                        command: None,
                        query: Some("get-account-summary".to_string()),
                        data: serde_yaml::Value::Null,
                        parameters: serde_yaml::from_str("id: acc-1").unwrap(),
                        actor: Some("customer".to_string()),
                    },
                    then: ThenExpectation {
                        events: None,
                        commands: None,
                        emits: None,
                        set_timers: None,
                        cancel_timers: None,
                        state: None,
                        ended: None,
                        rejection: None,
                        result: Some(serde_yaml::from_str("balance: 100").unwrap()),
                        read_model: None,
                    },
                },
                Scenario {
                    name: "rejects unauthorized query".to_string(),
                    description: None,
                    given: vec![],
                    when: WhenAction {
                        event: None,
                        bounded_context: None,
                        aggregate: None,
                        timer: None,
                        command: None,
                        query: Some("get-account-summary".to_string()),
                        data: serde_yaml::Value::Null,
                        parameters: serde_yaml::from_str("id: acc-1").unwrap(),
                        actor: Some("guest".to_string()),
                    },
                    then: ThenExpectation {
                        events: None,
                        commands: None,
                        emits: None,
                        set_timers: None,
                        cancel_timers: None,
                        state: None,
                        ended: None,
                        rejection: Some(RejectionExpectation {
                            invariant: None,
                            reason: Some("not authorized".to_string()),
                        }),
                        result: None,
                        read_model: None,
                    },
                },
            ]),
            ..Default::default()
        };

        let mut bc_contexts = HashMap::new();
        bc_contexts.insert(
            "finance".to_string(),
            BoundedContextGenerator {
                read_models: vec![rm_doc],
                features: vec![feature_doc],
                ..Default::default()
            },
        );
        let contexts = HashMap::new();
        let event_to_owner = HashMap::new();
        let routing = RoutingContext::default();
        let dir = tempfile::tempdir().unwrap();

        generate_tests(
            &dir.path().to_path_buf(),
            &contexts,
            &bc_contexts,
            &event_to_owner,
            &routing,
        )
        .unwrap();

        let bc_dir = resolve_bc_dir(dir.path(), "finance", &routing);
        let main_rs =
            std::fs::read_to_string(bc_dir.join("tests/generated/main.rs")).unwrap();
        let rm_test_rs = std::fs::read_to_string(
            bc_dir.join("tests/generated/read_model_account_summary.rs"),
        )
        .unwrap();

        assert!(main_rs.contains("mod read_model_account_summary;"));
        assert!(main_rs.contains("clippy::doc_markdown"));
        assert!(rm_test_rs.contains("/// Feature: `query-account-summary`"));
        assert!(rm_test_rs.contains("/// Scenario: `returns projected balance`"));
        assert!(rm_test_rs.contains("/// Projects balance updates after deposit."));
        assert!(rm_test_rs.contains("then_result_json"));
        assert!(rm_test_rs.contains("then_rejected_reason(\"not authorized\")"));
        assert!(
            rm_test_rs.contains("fn test_query_account_summary_returns_projected_balance()")
        );
    }
}
