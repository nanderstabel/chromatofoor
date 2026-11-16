use anyhow::Result;
use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ActiveTransports {
    pub rest: bool,
    pub cli: bool,
    pub graphql: bool,
    pub tauri: bool,
    pub mcp: bool,
}

pub fn generate_workspace(
    out_dir: &PathBuf,
    bc_aggregates: &HashMap<String, Vec<String>>,
    chromatofoor_path: &str,
    has_event_handlers: bool,
    has_policies: bool,
    bc_contexts: &HashMap<String, crate::generator::BoundedContextGenerator>,
    process_managers: &[crate::model::EsdmDoc],
    all_esdm_docs: &[crate::model::EsdmDoc],
    routing: &crate::generator::RoutingContext,
    active_transports: &ActiveTransports,
) -> Result<()> {
    let mut workspace_cargo = String::new();
    workspace_cargo.push_str("[workspace]\nresolver = \"2\"\nmembers = [\n");

    let mut members = Vec::new();
    members.push("    \"domain\"".to_string());
    let mut bcs: Vec<&String> = bc_aggregates.keys().filter(|&bc| bc != "global").collect();
    bcs.sort();
    for bc in bcs {
        let bc_dir = crate::generator::resolve_bc_dir(out_dir, bc, routing);
        let rel_path = crate::generator::get_relative_cargo_path(out_dir, &bc_dir);
        members.push(format!("    \"{rel_path}\""));
    }
    if has_event_handlers {
        members.push("    \"integration/event-handlers\"".to_string());
    }
    if has_policies {
        members.push("    \"integration/policies\"".to_string());
    }

    // Add process managers
    for pm in process_managers {
        members.push(format!("    \"integration/process-managers/{}\"", pm.name));
    }

    if active_transports.rest {
        members.push("    \"presentation/api-server\"".to_string());
    }
    if active_transports.graphql {
        members.push("    \"presentation/graphql-server\"".to_string());
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
        "chromatofoor-mcp = {{ path = \"{presentation_base}/mcp\" }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-tui = {{ path = \"{presentation_base}/tui\" }}\n"
    ));
    workspace_cargo.push_str(&format!(
        "chromatofoor-tauri = {{ path = \"{presentation_base}/tauri\" }}\n"
    ));

    fs::create_dir_all(out_dir)?;
    fs::write(out_dir.join("Cargo.toml"), workspace_cargo)?;

    // Generate shared `domain` crate with the canonical `#[chromatofoor::domain]` and `#[chromatofoor::external_system]` types
    let domain_crate_dir = out_dir.join("domain");
    let domain_crate_src = domain_crate_dir.join("src");
    fs::create_dir_all(&domain_crate_src)?;
    let domain_cargo_toml = r#"[package]
name = "domain"
version = "0.1.0"
edition = "2024"

[dependencies]
chromatofoor = { workspace = true }
"#;
    fs::write(domain_crate_dir.join("Cargo.toml"), domain_cargo_toml)?;

    let domain_pascal = routing.domain_ident(None);
    let domain_doc = all_esdm_docs
        .iter()
        .find(|doc| doc.kind == "domain")
        .and_then(|doc| doc.description.as_deref())
        .map_or_else(
            || quote! {},
            |d| crate::generator::make_doc_comment(Some(d)),
        );

    let mut domain_lib_chunks = vec![quote! {
        #domain_doc
        #[chromatofoor::domain]
        pub struct #domain_pascal;
    }];

    let mut external_systems: Vec<&crate::model::EsdmDoc> = all_esdm_docs
        .iter()
        .filter(|doc| doc.kind == "external-system")
        .collect();
    external_systems.sort_by_key(|doc| &doc.name);

    for ext in external_systems {
        let ext_pascal = format_ident!("{}", ext.name.to_case(Case::Pascal));
        let ext_doc = crate::generator::make_doc_comment(ext.description.as_deref());
        let dir_ident = match ext.direction.as_deref().unwrap_or("outbound") {
            "inbound" => format_ident!("inbound"),
            "bidirectional" => format_ident!("bidirectional"),
            _ => format_ident!("outbound"),
        };
        let cat_attr = if let Some(cat) = &ext.category {
            quote! { , category = #cat }
        } else {
            quote! {}
        };
        domain_lib_chunks.push(quote! {
            #ext_doc
            #[chromatofoor::external_system(
                scope = #domain_pascal,
                direction = #dir_ident
                #cat_attr
            )]
            #[derive(Clone, Copy, Default, Debug)]
            pub struct #ext_pascal;
        });
    }

    crate::generator::write_rust_file(
        &domain_crate_src.join("lib.rs"),
        quote! { #(#domain_lib_chunks)* },
    )?;

    let mut sorted_bcs: Vec<&String> = bc_aggregates.keys().collect();
    sorted_bcs.sort();

    for bc in sorted_bcs {
        if bc == "global" {
            continue;
        }
        let _aggregates = bc_aggregates
            .get(bc)
            .ok_or_else(|| anyhow::anyhow!("BC aggregates not found: {bc}"))?;

        let bc_dir = crate::generator::resolve_bc_dir(out_dir, bc, routing);
        fs::create_dir_all(&bc_dir)?;

        let mut extra_dependencies = String::new();
        let mut has_external_system = false;
        let mut external_system_deps = std::collections::HashSet::new();

        if let Some(generator) = bc_contexts.get(bc) {
            let mut deps = std::collections::HashSet::new();
            for mapping in &generator.context_mappings {
                // Determine if this bounded context is the dependent party
                let is_dependent = match mapping.doc_type.as_deref() {
                    Some("customer-supplier") => mapping
                        .customer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("conformist") => mapping
                        .conformist
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("anti-corruption-layer") => mapping
                        .downstream
                        .as_ref()
                        .is_some_and(|d| d.bounded_context.as_deref() == Some(bc)),
                    Some("open-host-service") => mapping
                        .consumer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    Some("published-language") => mapping
                        .consumer
                        .as_ref()
                        .is_some_and(|c| c.bounded_context.as_deref() == Some(bc)),
                    _ => false,
                };

                if is_dependent {
                    // Extract the target endpoint
                    let target = match mapping.doc_type.as_deref() {
                        Some("customer-supplier") => mapping.supplier.as_ref(),
                        Some("conformist" | "anti-corruption-layer") => mapping.upstream.as_ref(),
                        Some("open-host-service") => mapping.host.as_ref(),
                        Some("published-language") => mapping.publisher.as_ref(),
                        _ => None,
                    };

                    if let Some(target) = target {
                        if let Some(target_bc) = &target.bounded_context {
                            deps.insert(target_bc.clone());
                        } else if let Some(ext_sys) = &target.external_system {
                            has_external_system = true;
                            external_system_deps.insert(ext_sys.clone());
                        }
                    }
                }
            }

            let mut sorted_deps: Vec<_> = deps.into_iter().collect();
            sorted_deps.sort(); // Deterministic output
            for dep in sorted_deps {
                let target_dir = crate::generator::resolve_bc_dir(out_dir, &dep, routing);
                let rel_path = crate::generator::get_relative_cargo_path(&bc_dir, &target_dir);
                extra_dependencies.push_str(&format!("    {dep} = {{ path = \"{rel_path}\" }}\n"));
            }
        }

        let domain_rel_path = crate::generator::get_relative_cargo_path(&bc_dir, &domain_crate_dir);

        let cargo_toml = format!(
            r#"[package]
    name = "{bc}"
    version = "0.1.0"
    edition = "2024"
    
    [dependencies]
    chromatofoor = {{ workspace = true }}
    domain = {{ path = "{domain_rel_path}" }}
    anyhow = {{ workspace = true }}
    async-trait = {{ workspace = true }}
    serde = {{ version = "1.0", features = ["derive"] }}
    thiserror = "2.0"
    uuid = {{ version = "1.11", features = ["serde", "v4"] }}
    chrono = {{ version = "0.4", features = ["serde"] }}
    serde_json = {{ workspace = true }}
    tracing = {{ workspace = true }}
{extra_dependencies}
    [dev-dependencies]
    tokio = {{ workspace = true }}
    rstest = "0.23"
    serde_json = {{ workspace = true }}
    tracing-test = {{ workspace = true }}
    "#
        );
        fs::write(bc_dir.join("Cargo.toml"), cargo_toml)?;

        let src_dir = bc_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        let mut lib_rs_content =
            "pub mod generated;\npub mod domain;\npub mod application;\n".to_string();
        if has_external_system {
            fs::create_dir_all(src_dir.join("infrastructure"))?;
            let mod_rs_path = src_dir.join("infrastructure").join("mod.rs");
            if !mod_rs_path.exists() {
                fs::write(
                    &mod_rs_path,
                    "// External system integrations (e.g. ACLs, HTTP clients) go here\n",
                )?;
            }

            let mut sorted_exts: Vec<_> = external_system_deps.into_iter().collect();
            sorted_exts.sort();
            let mut ext_mod_names = Vec::new();

            for ext in sorted_exts {
                let snake_ext = ext.to_case(Case::Snake);
                ext_mod_names.push(snake_ext.clone());

                let file_path = src_dir
                    .join("infrastructure")
                    .join(format!("{snake_ext}.rs"));
                if !file_path.exists() {
                    let ext_doc_tokens = if let Some(generator) = bc_contexts.get(bc)
                        && let Some(ext_doc) =
                            generator.external_systems.iter().find(|d| d.name == ext)
                    {
                        crate::generator::make_doc_comment(ext_doc.description.as_deref())
                    } else {
                        quote! {}
                    };

                    let trait_name = format_ident!("{}", ext.to_case(Case::Pascal));
                    let trait_code = quote! {
                        use async_trait::async_trait;

                        #ext_doc_tokens
                        #[async_trait]
                        pub trait #trait_name: Send + Sync {}
                    };
                    crate::generator::write_manual_rust_file(&file_path, trait_code)?;
                }
            }
            crate::generator::ensure_manual_mod_file(&mod_rs_path, &ext_mod_names)?;
            lib_rs_content.push_str("pub mod infrastructure;\n");
        }

        fs::write(src_dir.join("lib.rs"), lib_rs_content)?;

        // Setup generated/ directory
        let gen_dir = src_dir.join("generated");
        fs::create_dir_all(&gen_dir)?;
        crate::generator::write_rust_file(
            &gen_dir.join("mod.rs"),
            quote! {
                pub mod application;
                pub mod bootstrap;
                pub mod domain;
                pub mod prelude;
            },
        )?;

        let gen_domain_dir = gen_dir.join("domain");
        fs::create_dir_all(&gen_domain_dir)?;

        let mut gen_domain_mod_chunks = Vec::new();
        let bc_pascal = format_ident!("{}", bc.to_case(Case::Pascal));
        gen_domain_mod_chunks.push(quote! {
            pub use ::domain::#domain_pascal;

            #[chromatofoor::bounded_context(scope = #domain_pascal)]
            pub struct #bc_pascal;
        });
        if let Some(aggregates) = bc_aggregates.get(bc) {
            let mut sorted_aggs = aggregates.clone();
            sorted_aggs.sort();
            for agg in &sorted_aggs {
                if agg == "global" {
                    continue;
                }
                let mod_name = format_ident!("{}", agg.to_case(Case::Snake));
                gen_domain_mod_chunks.push(quote! { pub mod #mod_name; });
            }
        }
        if let Some(ctx) = bc_contexts.get(bc) {
            if !ctx.entities.is_empty() {
                gen_domain_mod_chunks.push(quote! {
                    pub mod entities;
                    pub use entities::*;
                });
            }
            if !ctx.value_objects.is_empty() {
                gen_domain_mod_chunks.push(quote! {
                    pub mod value_objects;
                    pub use value_objects::*;
                });
            }
            if !ctx.domain_services.is_empty() {
                gen_domain_mod_chunks.push(quote! { pub mod services; });
            }
            if !ctx.actors.is_empty() {
                gen_domain_mod_chunks.push(quote! { pub mod actors; });
            }
        }
        let final_tokens = quote! { #(#gen_domain_mod_chunks)* };
        crate::generator::write_rust_file(&gen_domain_dir.join("mod.rs"), final_tokens)?;

        // Setup manual domain/ directory if it doesn't exist
        let domain_dir = src_dir.join("domain");
        if !domain_dir.exists() {
            fs::create_dir_all(&domain_dir)?;
        }

        // src/domain/mod.rs (manual)
        let domain_mod_path = domain_dir.join("mod.rs");
        let mut domain_mod_names = Vec::new();
        if let Some(aggregates) = bc_aggregates.get(bc) {
            let mut sorted_aggs = aggregates.clone();
            sorted_aggs.sort();
            for agg in &sorted_aggs {
                if agg == "global" {
                    continue;
                }
                domain_mod_names.push(agg.to_case(Case::Snake));
            }
        }
        if let Some(ctx) = bc_contexts.get(bc)
            && !ctx.domain_services.is_empty()
        {
            domain_mod_names.push("services".to_string());
        }
        crate::generator::ensure_manual_mod_file(&domain_mod_path, &domain_mod_names)?;
    }

    // Generate api-server
    if active_transports.rest {
        let api_server_dir = out_dir.join("presentation/api-server");
        fs::create_dir_all(&api_server_dir)?;

        let api_server_cargo = r#"[package]
name = "api-server"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
bootstrap = { path = "../../bootstrap" }
chromatofoor = { workspace = true }
chromatofoor-http = { workspace = true }
"#;
        fs::write(api_server_dir.join("Cargo.toml"), api_server_cargo)?;

        let api_server_src_dir = api_server_dir.join("src");
        fs::create_dir_all(&api_server_src_dir)?;
        let main_rs_content = r#"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    tracing::info!("Starting HTTP server on 0.0.0.0:3000");

    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_http::HttpServer::bind("0.0.0.0:3000"))
        .serve()
        .await
}
"#;
        fs::write(api_server_src_dir.join("main.rs"), main_rs_content)?;
    }

    // Generate graphql-server
    if active_transports.graphql {
        let graphql_server_dir = out_dir.join("presentation/graphql-server");
        fs::create_dir_all(&graphql_server_dir)?;

        let graphql_server_cargo = r#"[package]
name = "graphql-server"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
bootstrap = { path = "../../bootstrap" }
chromatofoor = { workspace = true }
chromatofoor-http = { workspace = true }
chromatofoor-graphql = { workspace = true }
"#;
        fs::write(graphql_server_dir.join("Cargo.toml"), graphql_server_cargo)?;

        let graphql_server_src_dir = graphql_server_dir.join("src");
        fs::create_dir_all(&graphql_server_src_dir)?;
        let graphql_main_rs_content = r#"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();
    tracing::info!("Starting GraphQL server on 0.0.0.0:8000");
    tracing::info!("GraphiQL UI available at http://localhost:8000/");

    bootstrap::Domain::build()
        .await?
        .attach(
            chromatofoor_http::HttpServer::bind("0.0.0.0:8000")
                .without_default_routes()
                .attach(chromatofoor_graphql::GraphQL::new()),
        )
        .serve()
        .await
}
"#;
        fs::write(
            graphql_server_src_dir.join("main.rs"),
            graphql_main_rs_content,
        )?;
    }

    // Generate mcp-server
    if active_transports.mcp {
        let mcp_server_dir = out_dir.join("presentation/mcp-server");
        fs::create_dir_all(&mcp_server_dir)?;

        let mcp_server_cargo = r#"[package]
name = "mcp-server"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
bootstrap = { path = "../../bootstrap" }
chromatofoor = { workspace = true }
chromatofoor-mcp = { workspace = true }
"#;
        fs::write(mcp_server_dir.join("Cargo.toml"), mcp_server_cargo)?;

        let mcp_server_src_dir = mcp_server_dir.join("src");
        fs::create_dir_all(&mcp_server_src_dir)?;
        let mcp_main_rs_content = r#"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    tracing::info!("Starting MCP server over stdio");

    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_mcp::Mcp::stdio())
        .serve()
        .await
}
"#;
        fs::write(mcp_server_src_dir.join("main.rs"), mcp_main_rs_content)?;
    }

    // Generate interactive-tui
    if active_transports.cli {
        let cli_server_dir = out_dir.join("presentation/interactive-tui");
        fs::create_dir_all(&cli_server_dir)?;

        let cli_server_cargo = r#"[package]
name = "interactive-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
bootstrap = { path = "../../bootstrap" }
chromatofoor-tui = { workspace = true }
chromatofoor = { workspace = true }
"#;
        fs::write(cli_server_dir.join("Cargo.toml"), cli_server_cargo)?;

        let cli_server_src_dir = cli_server_dir.join("src");
        fs::create_dir_all(&cli_server_src_dir)?;
        let cli_main_rs_content = r"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    bootstrap::Domain::build()
        .await?
        .attach(chromatofoor_tui::Tui)
        .serve()
        .await
}
";
        fs::write(cli_server_src_dir.join("main.rs"), cli_main_rs_content)?;
    }

    // Generate tauri-app
    if active_transports.tauri {
        let tauri_app_dir = out_dir.join("presentation/tauri-app");
        fs::create_dir_all(&tauri_app_dir)?;

        fs::write(
            tauri_app_dir.join("package.json"),
            include_str!("../../templates/tauri/package.json"),
        )?;
        fs::write(
            tauri_app_dir.join("tsconfig.json"),
            include_str!("../../templates/tauri/tsconfig.json"),
        )?;
        fs::write(
            tauri_app_dir.join("vite.config.ts"),
            include_str!("../../templates/tauri/vite.config.ts"),
        )?;
        let _ = fs::remove_file(tauri_app_dir.join("vite.config.js"));

        let src_js_dir = tauri_app_dir.join("src");
        fs::create_dir_all(&src_js_dir)?;
        fs::write(
            src_js_dir.join("index.css"),
            include_str!("../../templates/tauri/src/index.css"),
        )?;

        let public_dir = tauri_app_dir.join("public");
        fs::create_dir_all(&public_dir)?;

        if let Ok(logo_data) = fs::read("branding/logo-horizontal.jpg") {
            let _ = fs::write(public_dir.join("logo-horizontal.jpg"), logo_data);
        }
        if let Ok(logo_data) = fs::read("branding/logo-icon.jpg") {
            let _ = fs::write(public_dir.join("logo-icon.jpg"), logo_data);
        }

        fs::write(
            tauri_app_dir.join("index.html"),
            include_str!("../../templates/tauri/index.html"),
        )?;
        fs::write(
            src_js_dir.join("main.ts"),
            include_str!("../../templates/tauri/src/main.ts"),
        )?;

        let app_svelte_file = src_js_dir.join("App.svelte");
        if !app_svelte_file.exists() {
            fs::write(
                app_svelte_file,
                include_str!("../../templates/tauri/src/App.svelte"),
            )?;
        }

        let src_tauri_dir = tauri_app_dir.join("src-tauri");
        fs::create_dir_all(&src_tauri_dir)?;
        fs::write(
            src_tauri_dir.join("tauri.conf.json"),
            include_str!("../../templates/tauri/src-tauri/tauri.conf.json"),
        )?;
        fs::write(
            src_tauri_dir.join("Cargo.toml"),
            include_str!("../../templates/tauri/src-tauri/Cargo.toml"),
        )?;
        fs::write(
            src_tauri_dir.join("build.rs"),
            include_str!("../../templates/tauri/src-tauri/build.rs"),
        )?;

        let capabilities_dir = src_tauri_dir.join("capabilities");
        fs::create_dir_all(&capabilities_dir)?;
        fs::write(
            capabilities_dir.join("default.json"),
            include_str!("../../templates/tauri/src-tauri/capabilities/default.json"),
        )?;

        let src_tauri_src_dir = src_tauri_dir.join("src");
        fs::create_dir_all(&src_tauri_src_dir)?;

        let icons_dir = src_tauri_dir.join("icons");
        fs::create_dir_all(&icons_dir)?;
        let dummy_icon: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 8, 153, 99, 96, 0, 2, 0,
            0, 5, 0, 1, 250, 105, 185, 222, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];
        fs::write(icons_dir.join("icon.png"), dummy_icon)?;

        fs::write(
            src_tauri_src_dir.join("main.rs"),
            include_str!("../../templates/tauri/src-tauri/src/main.rs"),
        )?;
    }

    Ok(())
}
