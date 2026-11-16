#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = Arc::new(bootstrap::Domain::build().await?);

    let mut builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(
            tauri_plugin_mcp_bridge::Builder::new()
                .bind_address("127.0.0.1")
                .build(),
        );
    }

    builder
        .manage(chromatofoor_tauri::DomainState { dispatcher: root })
        .invoke_handler(tauri::generate_handler![
            chromatofoor_tauri::commands::execute_command,
            chromatofoor_tauri::commands::execute_query,
            chromatofoor_tauri::commands::dispatch_command,
            chromatofoor_tauri::commands::query_read_model,
            chromatofoor_tauri::commands::get_schema,
            chromatofoor_tauri::commands::get_events,
            chromatofoor_tauri::commands::window_control
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
