use crate::common::{BoundedContextGenerator, GeneratorContext, RoutingContext};
use crate::presentation::ActiveTransports;
use anyhow::{Context, Result};
use esdm::EsdmDoc;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct CodegenOptions {
    pub input: PathBuf,
    pub features: Option<PathBuf>,
    pub infra: Option<PathBuf>,
    pub lifecycle: Option<PathBuf>,
    pub out_dir: PathBuf,
    pub chromatofoor_path: String,
    pub emit_graph: Option<PathBuf>,
}

pub fn run_codegen(opts: CodegenOptions) -> Result<()> {
    let input = opts.input;
    let out_dir_arg = opts.out_dir;

    let yaml_content =
        fs::read_to_string(&input).with_context(|| format!("Failed to read file: {input:?}"))?;

    let docs = serde_yaml::Deserializer::from_str(&yaml_content);

    let mut all_esdm_docs = Vec::new();
    for doc in docs {
        let value = match serde_yaml::Value::deserialize(doc) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let esdm_doc: EsdmDoc = match serde_yaml::from_value(value.clone()) {
            Ok(d) => d,
            Err(e) => {
                println!(
                    "Warning: Skipping document. Could not parse as EsdmDoc: {e}\nValue: {value:?}"
                );
                continue;
            }
        };
        all_esdm_docs.push(esdm_doc);
    }

    // Load feature documents if explicitly provided or auto-discovered next to input
    if let Some(ref explicit_features) = opts.features
        && !explicit_features.is_file()
    {
        anyhow::bail!(
            "Explicitly specified feature file does not exist: {}",
            explicit_features.display()
        );
    }

    let features_path = opts.features.or_else(|| {
        let candidate = input.with_file_name("model.feature.esdm.yaml");
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    });

    if let Some(ref path) = features_path
        && path.is_file()
    {
        let feature_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read feature file: {path:?}"))?;

        let docs = serde_yaml::Deserializer::from_str(&feature_content);
        for doc in docs {
            let value = serde_yaml::Value::deserialize(doc).with_context(|| {
                format!("Failed to parse YAML document in feature file: {path:?}")
            })?;
            if value.is_null() {
                continue;
            }
            let esdm_doc: EsdmDoc = serde_yaml::from_value(value.clone()).with_context(|| {
                format!(
                    "Failed to deserialize document in feature file {path:?} as EsdmDoc.\nValue: {value:?}"
                )
            })?;
            all_esdm_docs.push(esdm_doc);
        }
    }

    // Load infrastructure documents if explicitly provided or auto-discovered next to input
    let infra_path = opts.infra.or_else(|| {
        let candidate = input.with_file_name("model.infra.esdm.yaml");
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    });

    let mut infra_docs = Vec::new();
    if let Some(ref path) = infra_path
        && path.is_file()
    {
        let infra_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read infra file: {path:?}"))?;

        let docs = serde_yaml::Deserializer::from_str(&infra_content);
        for doc in docs {
            let value = match serde_yaml::Value::deserialize(doc) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let esdm_doc: EsdmDoc = match serde_yaml::from_value(value.clone()) {
                Ok(d) => d,
                Err(e) => {
                    println!(
                        "Warning: Skipping document in infra file {path:?}. Could not parse as EsdmDoc: {e}\nValue: {value:?}"
                    );
                    continue;
                }
            };
            infra_docs.push(esdm_doc.clone());
            all_esdm_docs.push(esdm_doc);
        }
    }

    // Load lifecycle documents if explicitly provided or auto-discovered next to input
    if let Some(ref explicit_lifecycle) = opts.lifecycle
        && !explicit_lifecycle.is_file()
    {
        anyhow::bail!(
            "Explicitly specified lifecycle file does not exist: {}",
            explicit_lifecycle.display()
        );
    }

    let lifecycle_path = opts.lifecycle.or_else(|| {
        let candidate = input.with_file_name("model.lifecycle.esdm.yaml");
        if candidate.is_file() {
            Some(candidate)
        } else {
            None
        }
    });

    if let Some(ref path) = lifecycle_path
        && path.is_file()
    {
        let lifecycle_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read lifecycle file: {path:?}"))?;

        let docs = serde_yaml::Deserializer::from_str(&lifecycle_content);
        for doc in docs {
            let value = serde_yaml::Value::deserialize(doc).with_context(|| {
                format!("Failed to parse YAML document in lifecycle file: {path:?}")
            })?;
            if value.is_null() {
                continue;
            }
            let esdm_doc: EsdmDoc = serde_yaml::from_value(value.clone()).with_context(|| {
                format!(
                    "Failed to deserialize document in lifecycle file {path:?} as EsdmDoc.\nValue: {value:?}"
                )
            })?;
            all_esdm_docs.push(esdm_doc);
        }
    }

    let presentation_bindings: Vec<&EsdmDoc> = infra_docs
        .iter()
        .filter(|doc| doc.kind == "presentation-binding")
        .collect();

    let active_transports = if presentation_bindings.is_empty() {
        ActiveTransports {
            http: cfg!(feature = "http"),
            cli: cfg!(feature = "tui"),
            graphql: cfg!(feature = "graphql"),
            tauri: cfg!(feature = "tauri"),
            mcp: cfg!(feature = "mcp"),
        }
    } else {
        let mut transports = ActiveTransports::default();
        for doc in presentation_bindings {
            match doc.transport.as_deref() {
                Some("http" | "http-rest") => transports.http = true,
                Some("cli") => transports.cli = true,
                Some("graphql") => transports.graphql = true,
                Some("tauri-ipc") => transports.tauri = true,
                Some("mcp") => transports.mcp = true,
                _ => {}
            }
        }
        transports
    };

    let domain_name = all_esdm_docs
        .iter()
        .find(|doc| doc.kind == "domain")
        .map(|doc| doc.name.clone())
        .ok_or_else(|| anyhow::anyhow!("No domain document found in the input YAML"))?;

    let out_dir = out_dir_arg.join(&domain_name);

    let mut routing_context = RoutingContext {
        domain_name: domain_name.clone(),
        ..Default::default()
    };
    for doc in &all_esdm_docs {
        if doc.kind == "subdomain" {
            if let Some(covers) = &doc.bounded_contexts {
                for bc in covers {
                    routing_context
                        .bc_to_subdomain
                        .insert(bc.clone(), doc.name.clone());
                }
            }
        } else if doc.kind == "bounded-context"
            && let Some(metadata) = &doc.metadata
            && let Some(labels) = &metadata.labels
            && let Some(tier) = labels.get("tier")
        {
            routing_context
                .bc_to_tier
                .insert(doc.name.clone(), tier.clone());
        }
    }

    let mut event_to_owner: HashMap<String, (String, String)> = HashMap::new();
    for doc in &all_esdm_docs {
        if doc.kind == "command" {
            let bc = doc
                .scope
                .as_ref()
                .and_then(|s| s.bounded_context.clone())
                .unwrap_or_else(|| "global".to_string());
            let agg = doc
                .scope
                .as_ref()
                .and_then(|s| {
                    s.aggregate
                        .clone()
                        .or_else(|| s.dynamic_consistency_boundary.clone())
                })
                .unwrap_or_else(|| "global".to_string());

            if let Some(publishes) = &doc.publishes {
                for ev in publishes {
                    event_to_owner.insert(ev.clone(), (bc.clone(), agg.clone()));
                }
            }
        }
    }

    let mut contexts: HashMap<(String, String), GeneratorContext> = HashMap::new();
    let mut bc_contexts: HashMap<String, BoundedContextGenerator> = HashMap::new();
    let mut event_handlers: Vec<EsdmDoc> = Vec::new();
    let mut process_managers: Vec<EsdmDoc> = Vec::new();
    let mut policies: Vec<EsdmDoc> = Vec::new();

    for esdm_doc in &all_esdm_docs {
        let is_agg_feature = esdm_doc.kind == "feature"
            && esdm_doc
                .scope
                .as_ref()
                .is_some_and(|s| s.aggregate.is_some() || s.dynamic_consistency_boundary.is_some());
        let is_bc_feature = esdm_doc.kind == "feature"
            && esdm_doc
                .scope
                .as_ref()
                .is_some_and(|s| s.read_model.is_some() || s.process_manager.is_some());

        if esdm_doc.kind == "command"
            || esdm_doc.kind == "event"
            || esdm_doc.kind == "aggregate"
            || esdm_doc.kind == "dynamic-consistency-boundary"
            || esdm_doc.kind == "aggregate-lifecycle"
            || is_agg_feature
        {
            let mut bc = esdm_doc
                .scope
                .as_ref()
                .and_then(|s| s.bounded_context.clone())
                .unwrap_or_else(|| "global".to_string());

            let agg = if esdm_doc.kind == "aggregate"
                || esdm_doc.kind == "dynamic-consistency-boundary"
            {
                esdm_doc.name.clone()
            } else if esdm_doc.kind == "command"
                || esdm_doc.kind == "aggregate-lifecycle"
                || is_agg_feature
            {
                esdm_doc
                    .scope
                    .as_ref()
                    .and_then(|s| {
                        s.aggregate
                            .clone()
                            .or_else(|| s.dynamic_consistency_boundary.clone())
                    })
                    .unwrap_or_else(|| "global".to_string())
            } else if let Some(explicit_agg) = esdm_doc.scope.as_ref().and_then(|s| {
                s.aggregate
                    .clone()
                    .or_else(|| s.dynamic_consistency_boundary.clone())
            }) {
                explicit_agg
            } else if let Some((mapped_bc, mapped_agg)) = event_to_owner.get(&esdm_doc.name) {
                bc = mapped_bc.clone();
                mapped_agg.clone()
            } else {
                "global".to_string()
            };

            let ctx = contexts.entry((bc.clone(), agg.clone())).or_default();
            if esdm_doc.kind == "command" {
                ctx.commands.push(esdm_doc.clone());
            } else if esdm_doc.kind == "event" {
                ctx.events.push(esdm_doc.clone());
            } else if esdm_doc.kind == "aggregate" {
                ctx.aggregate = Some(esdm_doc.clone());
            } else if esdm_doc.kind == "dynamic-consistency-boundary" {
                ctx.boundary = Some(esdm_doc.clone());
            } else if esdm_doc.kind == "aggregate-lifecycle" {
                if let Some(prev) = &ctx.lifecycle {
                    anyhow::bail!(
                        "Duplicate aggregate-lifecycle '{}' for aggregate '{}/{}' (already defined by '{}')",
                        esdm_doc.name,
                        bc,
                        agg,
                        prev.name
                    );
                }
                ctx.lifecycle = Some(esdm_doc.clone());
            } else if is_agg_feature {
                ctx.features.push(esdm_doc.clone());
            }
        } else if esdm_doc.kind == "bounded-context" {
            let bc_ctx = bc_contexts.entry(esdm_doc.name.clone()).or_default();
            if esdm_doc.description.is_some() {
                bc_ctx.description.clone_from(&esdm_doc.description);
            }
        } else if esdm_doc.kind == "read-model"
            || esdm_doc.kind == "query"
            || esdm_doc.kind == "domain-service"
            || esdm_doc.kind == "actor"
            || is_bc_feature
        {
            let bc = esdm_doc
                .scope
                .as_ref()
                .and_then(|s| s.bounded_context.clone())
                .unwrap_or_else(|| "global".to_string());

            let bc_ctx = bc_contexts.entry(bc).or_default();
            if esdm_doc.kind == "read-model" {
                bc_ctx.read_models.push(esdm_doc.clone());
            } else if esdm_doc.kind == "query" {
                bc_ctx.queries.push(esdm_doc.clone());
            } else if esdm_doc.kind == "domain-service" {
                bc_ctx.domain_services.push(esdm_doc.clone());
            } else if esdm_doc.kind == "actor" {
                bc_ctx.actors.push(esdm_doc.clone());
            } else if is_bc_feature {
                bc_ctx.features.push(esdm_doc.clone());
            }
        } else if esdm_doc.kind == "entity" || esdm_doc.kind == "value-object" {
            let bc = esdm_doc
                .scope
                .as_ref()
                .and_then(|s| s.bounded_context.clone())
                .unwrap_or_else(|| "global".to_string());
            let agg = esdm_doc.scope.as_ref().and_then(|s| {
                s.aggregate
                    .clone()
                    .or_else(|| s.dynamic_consistency_boundary.clone())
            });

            if let Some(agg_name) = agg {
                let ctx = contexts.entry((bc, agg_name)).or_default();
                if esdm_doc.kind == "entity" {
                    ctx.entities.push(esdm_doc.clone());
                } else {
                    ctx.value_objects.push(esdm_doc.clone());
                }
            } else {
                let bc_ctx = bc_contexts.entry(bc).or_default();
                if esdm_doc.kind == "entity" {
                    bc_ctx.entities.push(esdm_doc.clone());
                } else {
                    bc_ctx.value_objects.push(esdm_doc.clone());
                }
            }
        } else if esdm_doc.kind == "event-handler" {
            event_handlers.push(esdm_doc.clone());
        } else if esdm_doc.kind == "policy" {
            policies.push(esdm_doc.clone());
        } else if esdm_doc.kind == "process-manager" {
            process_managers.push(esdm_doc.clone());
        }
    }

    let lint_violations = esdm::lint::validate_model(&all_esdm_docs);
    if !lint_violations.is_empty() {
        let msgs: Vec<String> = lint_violations.iter().map(ToString::to_string).collect();
        anyhow::bail!(
            "ESDM model validation failed with {} violation(s):\n  {}",
            msgs.len(),
            msgs.join("\n  ")
        );
    }

    let external_systems: Vec<EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "external-system")
        .cloned()
        .collect();

    let context_mappings: Vec<EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "context-mapping")
        .cloned()
        .collect();

    for bc_ctx in bc_contexts.values_mut() {
        bc_ctx.context_mappings = context_mappings.clone();
        bc_ctx.external_systems = external_systems.clone();
    }
    let mut bc_aggregates: HashMap<String, Vec<String>> = HashMap::new();
    for (bc, agg) in contexts.keys() {
        bc_aggregates
            .entry(bc.clone())
            .or_default()
            .push(agg.clone());
    }
    for bc in bc_contexts.keys() {
        bc_aggregates.entry(bc.clone()).or_default();
    }

    crate::runtime::workspace::generate_workspace(
        &out_dir,
        &bc_aggregates,
        &opts.chromatofoor_path,
        &contexts,
        &bc_contexts,
        &event_handlers,
        &policies,
        &process_managers,
        &all_esdm_docs,
        &event_to_owner,
        &routing_context,
        &active_transports,
    )?;

    if let Some(ref graph_path) = opts.emit_graph {
        println!("Emitting domain graph to {graph_path:?}...");
        crate::graph::emit_graph_file(&all_esdm_docs, &event_to_owner, graph_path)?;
    }

    println!("Running cargo fmt...");
    let fmt_status = std::process::Command::new("cargo")
        .arg("fmt")
        .current_dir(&out_dir)
        .status();

    if let Err(e) = fmt_status {
        eprintln!("Warning: Failed to run cargo fmt: {e}");
    }

    println!("Code generation complete.");
    Ok(())
}
