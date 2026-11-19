use crate::common::MEMBER_LINTS_TOML;
use anyhow::Result;
use convert_case::{Case, Casing};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct ActiveTransports {
    pub http: bool,
    pub cli: bool,
    pub graphql: bool,
    pub tauri: bool,
    pub mcp: bool,
}

fn resolve_tauri_ui_path(chromatofoor_path: &str) -> String {
    let trimmed = chromatofoor_path.trim_end_matches('/');
    let repo_root = Path::new(trimmed)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if repo_root.is_absolute() {
        repo_root
            .join("packages/ui")
            .to_string_lossy()
            .replace('\\', "/")
    } else {
        Path::new("../..")
            .join(repo_root)
            .join("packages/ui")
            .to_string_lossy()
            .replace('\\', "/")
    }
}

pub fn generate_transports(
    out_dir: &Path,
    active_transports: &ActiveTransports,
    chromatofoor_path: &str,
    domain_name: Option<&str>,
) -> Result<()> {
    if active_transports.http || active_transports.graphql || active_transports.mcp {
        let api_server_dir = out_dir.join("presentation/api-server");
        fs::create_dir_all(&api_server_dir)?;

        let mut extra_deps = String::new();
        if active_transports.graphql {
            extra_deps.push_str("chromatofoor-graphql = { workspace = true }\n");
        }
        if active_transports.mcp {
            extra_deps.push_str("chromatofoor-mcp = { workspace = true }\n");
        }

        let api_server_cargo = format!(
            r#"[package]
name = "api-server"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
bootstrap = {{ path = "../../bootstrap" }}
chromatofoor = {{ workspace = true }}
chromatofoor-http = {{ workspace = true }}
{extra_deps}{MEMBER_LINTS_TOML}"#
        );
        fs::write(api_server_dir.join("Cargo.toml"), api_server_cargo)?;

        let api_server_src_dir = api_server_dir.join("src");
        fs::create_dir_all(&api_server_src_dir)?;

        let mut server_chain =
            String::from("chromatofoor_http::HttpServer::bind(\"0.0.0.0:3000\")");
        if !active_transports.http {
            server_chain.push_str("\n                .without_default_routes()");
        }
        if active_transports.graphql {
            if active_transports.http {
                server_chain
                    .push_str("\n                .attach(chromatofoor_graphql::GraphQL::new())");
            } else {
                server_chain.push_str(
                    "\n                .attach(chromatofoor_graphql::GraphQL::new().with_root())",
                );
            }
        }
        if active_transports.mcp {
            server_chain.push_str("\n                .attach(chromatofoor_mcp::Mcp::http())");
        }

        let mut startup_logs =
            String::from("    tracing::info!(\"Starting HTTP server on 0.0.0.0:3000\");\n");
        if active_transports.http {
            startup_logs.push_str(
                "    tracing::info!(\"Workbench UI available at http://localhost:3000/\");\n",
            );
        }
        if active_transports.graphql {
            let graphiql_path = if active_transports.http {
                "/graphiql"
            } else {
                "/"
            };
            startup_logs.push_str(&format!(
                "    tracing::info!(\"GraphiQL UI available at http://localhost:3000{graphiql_path}\");\n"
            ));
        }
        if active_transports.mcp {
            startup_logs.push_str(
                "    tracing::info!(\"MCP endpoint available at http://localhost:3000/mcp\");\n",
            );
        }

        let main_rs_content = format!(
            r"#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {{
    tracing_subscriber::fmt().init();
{startup_logs}
    bootstrap::Domain::build()
        .await?
        .attach(
            {server_chain},
        )
        .serve()
        .await
}}
"
        );
        fs::write(api_server_src_dir.join("main.rs"), main_rs_content)?;
    }

    if active_transports.mcp {
        let mcp_server_dir = out_dir.join("presentation/mcp-server");
        fs::create_dir_all(&mcp_server_dir)?;

        let mcp_server_cargo = format!(
            r#"[package]
name = "mcp-server"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
bootstrap = {{ path = "../../bootstrap" }}
chromatofoor = {{ workspace = true }}
chromatofoor-mcp = {{ workspace = true }}
{MEMBER_LINTS_TOML}"#
        );
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

    if active_transports.cli {
        let cli_server_dir = out_dir.join("presentation/interactive-tui");
        fs::create_dir_all(&cli_server_dir)?;

        let cli_server_cargo = format!(
            r#"[package]
name = "interactive-tui"
version = "0.1.0"
edition = "2024"

[dependencies]
tokio = {{ workspace = true }}
tracing = {{ workspace = true }}
tracing-subscriber = {{ workspace = true }}
bootstrap = {{ path = "../../bootstrap" }}
chromatofoor-tui = {{ workspace = true }}
chromatofoor = {{ workspace = true }}
{MEMBER_LINTS_TOML}"#
        );
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

    if active_transports.tauri {
        let tauri_app_dir = out_dir.join("presentation/tauri-app");
        fs::create_dir_all(&tauri_app_dir)?;

        let ui_rel_path = resolve_tauri_ui_path(chromatofoor_path);
        let pkg_json = include_str!("../../templates/tauri/package.json")
            .replace("../../../../packages/ui", &ui_rel_path);
        let vite_cfg = include_str!("../../templates/tauri/vite.config.ts")
            .replace("../../../../packages/ui", &ui_rel_path);

        fs::write(tauri_app_dir.join("package.json"), pkg_json)?;
        fs::write(
            tauri_app_dir.join("tsconfig.json"),
            include_str!("../../templates/tauri/tsconfig.json"),
        )?;
        fs::write(tauri_app_dir.join("vite.config.ts"), vite_cfg)?;
        let _ = fs::remove_file(tauri_app_dir.join("vite.config.js"));

        let dist_dir = tauri_app_dir.join("dist");
        fs::create_dir_all(&dist_dir)?;
        let dist_index = dist_dir.join("index.html");
        let is_legacy_stub =
            fs::read_to_string(&dist_index).is_ok_and(|content| content.contains("<body></body>"));
        if !dist_index.exists() || is_legacy_stub {
            let dist_assets_dir = dist_dir.join("assets");
            fs::create_dir_all(&dist_assets_dir)?;
            fs::write(
                dist_index,
                include_str!("../../../presentation/http/ui-dist/index.html"),
            )?;
            fs::write(
                dist_assets_dir.join("workbench.js"),
                include_str!("../../../presentation/http/ui-dist/assets/workbench.js"),
            )?;
            fs::write(
                dist_assets_dir.join("workbench.css"),
                include_str!("../../../presentation/http/ui-dist/assets/workbench.css"),
            )?;
            fs::write(
                dist_dir.join("logo-horizontal.jpg"),
                include_bytes!("../../../presentation/http/ui-dist/logo-horizontal.jpg"),
            )?;
            fs::write(
                dist_dir.join("logo-icon.jpg"),
                include_bytes!("../../../branding/logo-icon.jpg"),
            )?;
        }

        let src_js_dir = tauri_app_dir.join("src");
        fs::create_dir_all(&src_js_dir)?;
        fs::write(
            src_js_dir.join("index.css"),
            include_str!("../../templates/tauri/src/index.css"),
        )?;

        let public_dir = tauri_app_dir.join("public");
        fs::create_dir_all(&public_dir)?;
        fs::write(
            public_dir.join("logo-horizontal.jpg"),
            include_bytes!("../../../branding/logo-horizontal.jpg"),
        )?;
        fs::write(
            public_dir.join("logo-icon.jpg"),
            include_bytes!("../../../branding/logo-icon.jpg"),
        )?;

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

        let domain_slug = domain_name.map_or_else(|| "app".to_string(), |d| d.to_case(Case::Kebab));
        let window_title =
            domain_name.map_or_else(|| "Chromatofoor".to_string(), |d| d.to_case(Case::Title));
        let tauri_conf = include_str!("../../templates/tauri/src-tauri/tauri.conf.json")
            .replace("__PRODUCT_NAME__", &domain_slug)
            .replace("__DOMAIN_NAME__", &domain_slug)
            .replace("__WINDOW_TITLE__", &window_title);

        let src_tauri_dir = tauri_app_dir.join("src-tauri");
        fs::create_dir_all(&src_tauri_dir)?;
        fs::write(src_tauri_dir.join("tauri.conf.json"), tauri_conf)?;
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
        fs::write(
            capabilities_dir.join("desktop.json"),
            include_str!("../../templates/tauri/src-tauri/capabilities/desktop.json"),
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
            src_tauri_src_dir.join("lib.rs"),
            include_str!("../../templates/tauri/src-tauri/src/lib.rs"),
        )?;
        fs::write(
            src_tauri_src_dir.join("main.rs"),
            include_str!("../../templates/tauri/src-tauri/src/main.rs"),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transports_emit_workspace_lints_and_tauri_dist() {
        let dir = tempfile::tempdir().unwrap();
        let active = ActiveTransports {
            http: true,
            cli: true,
            graphql: true,
            tauri: true,
            mcp: true,
        };
        generate_transports(
            dir.path(),
            &active,
            "../../../../chromatofoor",
            Some("cargo-shipping"),
        )
        .unwrap();

        for rel in [
            "presentation/api-server/Cargo.toml",
            "presentation/mcp-server/Cargo.toml",
            "presentation/interactive-tui/Cargo.toml",
            "presentation/tauri-app/src-tauri/Cargo.toml",
        ] {
            let content = fs::read_to_string(dir.path().join(rel)).unwrap();
            assert!(
                content.contains("[lints]\nworkspace = true"),
                "missing [lints] in {rel}"
            );
        }

        let api_cargo =
            fs::read_to_string(dir.path().join("presentation/api-server/Cargo.toml")).unwrap();
        assert!(api_cargo.contains("chromatofoor-graphql"));
        assert!(api_cargo.contains("chromatofoor-mcp"));
        assert!(!dir.path().join("presentation/graphql-server").exists());

        assert!(
            dir.path()
                .join("presentation/tauri-app/dist/index.html")
                .is_file()
        );
        assert!(
            dir.path()
                .join("presentation/tauri-app/dist/assets/workbench.js")
                .is_file()
        );
        assert!(
            dir.path()
                .join("presentation/tauri-app/src-tauri/src/lib.rs")
                .is_file()
        );
        assert!(
            dir.path()
                .join("presentation/tauri-app/src-tauri/capabilities/desktop.json")
                .is_file()
        );
        let tauri_conf = fs::read_to_string(
            dir.path()
                .join("presentation/tauri-app/src-tauri/tauri.conf.json"),
        )
        .unwrap();
        assert!(tauri_conf.contains("\"productName\": \"cargo-shipping\""));
        assert!(tauri_conf.contains("\"identifier\": \"com.chromatofoor.cargo-shipping\""));
        assert!(tauri_conf.contains("\"title\": \"Cargo Shipping\""));

        let pkg =
            fs::read_to_string(dir.path().join("presentation/tauri-app/package.json")).unwrap();
        assert!(pkg.contains("../../../../../../packages/ui"));
        assert!(pkg.contains("\"android:init\""));
    }
}
