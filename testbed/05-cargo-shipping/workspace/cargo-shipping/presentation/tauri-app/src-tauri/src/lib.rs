use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let root = Arc::new(
        tauri::async_runtime::block_on(bootstrap::Domain::build())
            .expect("failed to initialize domain runtime"),
    );

    let builder = tauri::Builder::default();

    #[cfg(all(debug_assertions, not(any(target_os = "android", target_os = "ios"))))]
    let builder = builder.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

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
}
