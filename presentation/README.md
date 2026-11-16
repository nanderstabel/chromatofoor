# Presentation Adapters (`presentation`)

## Overview
Composable transport and user-interface adapters for `chromatofoor::domain::DomainRuntime`. Standalone process transports implement `chromatofoor::runtime::Presentation` (`HttpServer`, `McpStdio`, `Tui`), HTTP sub-routers implement `chromatofoor_http::HttpProtocol` (`GraphQL`, `McpHttp`) to share a single Axum listener, and desktop apps bind `chromatofoor_tauri::DomainState` + IPC commands directly into Tauri v2.

## Subdirectories & Boundaries
- `http/`: [`./http/README.md`](./http/README.md) — `chromatofoor-http`: Axum 0.8 HTTP (`GET`, `POST`, `SSE`) + OpenAPI 3.0 server (`HttpServer`) and composable `HttpProtocol` sub-router host.
- `graphql/`: [`./graphql/README.md`](./graphql/README.md) — `chromatofoor-graphql`: Dynamic `async-graphql` schema (`Query`, `Mutation`, `Subscription`), GraphiQL, SSE, and WebSocket `HttpProtocol` adapter.
- `mcp/`: [`./mcp/README.md`](./mcp/README.md) — `chromatofoor-mcp`: Model Context Protocol JSON-RPC 2.0 server exposing actor-checked domain tools and `esdm://` resources over `stdio` (`Presentation`) and Streamable HTTP + SSE (`HttpProtocol`).
- `tui/`: [`./tui/README.md`](./tui/README.md) — `chromatofoor-tui`: Interactive Ratatui/Crossterm split-pane terminal UI (`Presentation`) with actor switching, schema-seeded editors, and live event streaming.
- `tauri/`: [`./tauri/README.md`](./tauri/README.md) — `chromatofoor-tauri`: Tauri v2 managed `DomainState` and seven `#[tauri::command]` desktop IPC handlers for commands, queries, schema introspection, event snapshots, and window controls.
