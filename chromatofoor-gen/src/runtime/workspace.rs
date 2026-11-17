use crate::common::{
    BoundedContextGenerator, GeneratorContext, MEMBER_LINTS_TOML, RoutingContext,
    get_relative_cargo_path, resolve_bc_dir,
};
use crate::presentation::ActiveTransports;
use anyhow::Result;
use esdm::EsdmDoc;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn generate_workspace(
    out_dir: &PathBuf,
    bc_aggregates: &HashMap<String, Vec<String>>,
    chromatofoor_path: &str,
    contexts: &HashMap<(String, String), GeneratorContext>,
    bc_contexts: &HashMap<String, BoundedContextGenerator>,
    event_handlers: &[EsdmDoc],
    policies: &[EsdmDoc],
    process_managers: &[EsdmDoc],
    all_esdm_docs: &[EsdmDoc],
    event_to_owner: &HashMap<String, (String, String)>,
    routing: &RoutingContext,
    active_transports: &ActiveTransports,
) -> Result<()> {
    let has_event_handlers = !event_handlers.is_empty();
    let has_policies = !policies.is_empty();

    let mut workspace_cargo = String::new();
    workspace_cargo.push_str("[workspace]\nresolver = \"2\"\nmembers = [\n");

    let mut members = Vec::new();
    members.push("    \"domain\"".to_string());
    let mut bcs: Vec<&String> = bc_aggregates.keys().filter(|&bc| bc != "global").collect();
    bcs.sort();
    for bc in bcs {
        let bc_dir = resolve_bc_dir(out_dir, bc, routing);
        let rel_path = get_relative_cargo_path(out_dir, &bc_dir);
        members.push(format!("    \"{rel_path}\""));
    }
    if has_event_handlers {
        members.push("    \"integration/event-handlers\"".to_string());
    }
    if has_policies {
        members.push("    \"integration/policies\"".to_string());
    }

    for pm in process_managers {
        members.push(format!("    \"integration/process-managers/{}\"", pm.name));
    }

    if active_transports.http || active_transports.graphql || active_transports.mcp {
        members.push("    \"presentation/api-server\"".to_string());
    }
    if active_transports.mcp {
        members.push("    \"presentation/mcp-server\"".to_string());
    }
    if active_transports.cli {
        members.push("    \"presentation/interactive-tui\"".to_string());
    }
    if active_transports.tauri || out_dir.join("presentation/tauri-app/src-tauri").exists() {
        members.push("    \"presentation/tauri-app/src-tauri\"".to_string());
    }

    members.push("    \"bootstrap\"".to_string());
    members.sort();
    for member in members {
        workspace_cargo.push_str(&format!("{member},\n"));
    }
    workspace_cargo.push_str("]\n\n[workspace.dependencies]\n");
    workspace_cargo.push_str("anyhow = \"1.0\"\n");
    workspace_cargo.push_str("async-trait = \"0.1\"\n");
    workspace_cargo.push_str("tokio = { version = \"1.40\", features = [\"full\"] }\n");
    workspace_cargo.push_str("serde_json = \"1.0\"\n");
    workspace_cargo.push_str("tracing = \"0.1\"\n");
    workspace_cargo.push_str("tracing-subscriber = \"0.3\"\n");
    workspace_cargo.push_str("tracing-test = \"0.2\"\n");
    let trimmed_chromatofoor_path = chromatofoor_path.trim_end_matches('/');
    let presentation_base = match trimmed_chromatofoor_path.rsplit_once('/') {
        Some((parent, _)) if !parent.is_empty() => format!("{parent}/presentation"),
        Some(_) => "/presentation".to_string(),
        None => "presentation".to_string(),
    };
    workspace_cargo.push_str(&format!(
        "chromatofoor = {{ path = \"{chromatofoor_path}\" }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-http = {{ path = \"{presentation_base}/http\" }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-graphql = {{ path = \"{presentation_base}/graphql\", features = [\"axum\"] }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-mcp = {{ path = \"{presentation_base}/mcp\", features = [\"http\"] }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-tui = {{ path = \"{presentation_base}/tui\" }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-tauri = {{ path = \"{presentation_base}/tauri\" }}\n"
    ));
    workspace_cargo.push_str(
        "\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n\n[workspace.lints.clippy]\ntodo = \"warn\"\n",
    );

    fs::create_dir_all(out_dir)?;
    fs::write(out_dir.join("Cargo.toml"), workspace_cargo)?;

    let retained_tauri_cargo = out_dir.join("presentation/tauri-app/src-tauri/Cargo.toml");
    if !active_transports.tauri && retained_tauri_cargo.is_file() {
        let mut tauri_cargo = fs::read_to_string(&retained_tauri_cargo)?;
        if !tauri_cargo.contains("[lints]") {
            tauri_cargo.push_str(MEMBER_LINTS_TOML);
            fs::write(&retained_tauri_cargo, tauri_cargo)?;
        }
    }

    crate::domain::generate_domain_crate(out_dir, all_esdm_docs, routing)?;
    crate::bounded_context::generate_bounded_context_scaffolds(
        out_dir,
        bc_aggregates,
        bc_contexts,
        routing,
    )?;
    crate::presentation::generate_transports(out_dir, active_transports, chromatofoor_path)?;
    crate::bounded_context::generate_consistency_units(out_dir, contexts, bc_contexts, routing)?;
    crate::bounded_context::generate_application(out_dir, bc_contexts, contexts, routing)?;
    crate::bounded_context::generate_preludes(out_dir, bc_contexts, contexts, routing)?;
    crate::bounded_context::generate_domain_services(out_dir, bc_contexts, routing)?;
    crate::runtime::bootstrap::generate_bootstrap_crate(
        out_dir,
        bc_contexts,
        contexts,
        event_handlers,
        policies,
        process_managers,
        all_esdm_docs,
        routing,
    )?;
    crate::testing::generate_tests(out_dir, contexts, bc_contexts, event_to_owner, routing)?;
    crate::integration::generate_event_handlers(out_dir, event_handlers, routing)?;
    crate::integration::generate_policies(out_dir, policies, routing)?;
    crate::integration::generate_process_managers(out_dir, process_managers, routing)?;
    crate::testing::generate_process_manager_tests(out_dir, process_managers, all_esdm_docs)?;
    #[cfg(feature = "typescript")]
    {
        println!("Generating TypeScript interfaces...");
        crate::presentation::generate_typescript(out_dir, all_esdm_docs)?;
    }

    Ok(())
}
