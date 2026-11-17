# Chromatofoor MCP Presentation Adapter (`chromatofoor-mcp`)

## Overview
`chromatofoor-mcp` exposes any `chromatofoor::domain::DomainRuntime` as a **Model Context Protocol (MCP)** JSON-RPC 2.0 server built on `rmcp` (supporting MCP `2026-07-28` `server/discover` as well as `2025-11-25` and `2024-11-05` `initialize`). It automatically projects registered domain commands and queries into actor-checked MCP tools (`tools/list`, `tools/call`) and `esdm://` resources (`resources/list`, `resources/templates/list`, `resources/read`) over standalone `stdio` (`Presentation`) or composable Axum Streamable HTTP + live SSE (`HttpProtocol`).

## Interface
- **Cargo Feature Flags** (`Cargo.toml`):
  - `default = []` — Standalone `stdio` MCP adapter with zero HTTP server dependencies.
  - `http` — Enables `dep:axum`, `dep:chromatofoor-http`, and `dep:tokio-stream` for Streamable HTTP and live domain-event SSE (`Mcp::http()`, `McpHttp`, `routes`).
  - `axum` — Alias enabling `http`.
- **Composable Transport Entry Points** (`src/lib.rs`):
  - `Mcp::stdio() -> McpStdio` (`pub const fn`) — Standalone `stdio` presentation adapter implementing `chromatofoor::runtime::Presentation` (`name() -> "mcp-stdio"`).
  - `Mcp::http() -> McpHttp` (`#[cfg(feature = "http")]`) — Streamable HTTP sub-router adapter (`McpHttp::new()`, `with_path`, defaulting to `"/mcp"`) implementing `chromatofoor_http::HttpProtocol` (`name() -> "mcp-http"`) for mounting onto `chromatofoor_http::HttpServer`.
  - `routes(runtime: Arc<DomainRuntime>) -> axum::Router` (`#[cfg(feature = "http")]`) — Builds the `/mcp` Axum router (`POST /mcp` JSON-RPC single/batch + SSE response negotiation, and `GET /mcp` live `notifications/domain_event` SSE stream).
  - `async fn serve_mcp(runtime: Arc<DomainRuntime>) -> Result<(), anyhow::Error>` — Serves `McpDomainServer` over `rmcp::transport::stdio_pipes()`.
- **MCP Handler & JSON-RPC Dispatchers** (`src/lib.rs`):
  - `McpDomainServer` — `Clone` server (`new`, `runtime`) implementing `rmcp::ServerHandler` with `tools` and `resources` capabilities enabled.
  - `async fn handle_request(runtime: &DomainRuntime, req: &Value) -> Option<Value>` — Dispatches a raw JSON-RPC 2.0 request with `default_actor = None` (returning `None` for `"notifications/*"` methods).
  - `async fn handle_request_with_actor(runtime: &DomainRuntime, req: &Value, default_actor: Option<&str>) -> Option<Value>` — Dispatches `initialize`, `server/discover` (and `discover`), `notifications/*`, `tools/list`, `tools/call`, `resources/list`, `resources/templates/list`, and `resources/read` with fallback actor resolution.
- **Schema & Catalog Builders** (`src/lib.rs`):
  - `build_mcp_tools`, `list_tools` — Builds typed `rmcp::model::Tool` definitions (or JSON `Value`s) for commands (`invoke_{bounded_context}_{target}_{name}`) and queries (`query_{bounded_context}_{name}`).
  - `build_mcp_resources`, `build_mcp_resource_templates`, `list_resources`, `list_resource_templates` — Exposes `esdm://catalog` (`application/json`) alongside `esdm://{bounded_context}/{query_name}` resources (`list_resources` also embeds `"uriTemplate": "esdm://{bc}/{query}/{id}"` on query entries) and `esdm://{bounded_context}/{query_name}/{id}` URI templates.

## Invariants
- **Strict Rust Safety & Lint Baseline** (`Cargo.toml`): Inherits workspace lints enforcing `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)`.
- **Capabilities Surface** (`src/lib.rs:McpDomainServer`, `src/lib.rs:handle_request_with_actor`): Advertises only `tools` and `resources` capabilities; `prompts/*` methods are unsupported (`-32601 Method not found`).
- **Deterministic Tool Naming & Annotations** (`src/lib.rs:build_mcp_tools`):
  - Commands map to `invoke_{bounded_context}_{target}_{name}` with `readOnlyHint = false`, `destructiveHint = false`, `openWorldHint = false`.
  - Queries map to `query_{bounded_context}_{name}` with `readOnlyHint = true`, `idempotentHint = true`, `openWorldHint = false`, and attach `outputSchema` only when the query's output JSON Schema has `"type": "object"` (`structuredContent` is emitted only when the runtime JSON result `is_object()`).
- **Actor Extraction, Payload Unwrapping & Precedence** (`src/lib.rs:enrich_schema_with_actor`, `src/lib.rs:execute_tool_call`, `src/lib.rs:routes_at`):
  - Every generated tool input schema injects a top-level `_actor` property (`enum` of `allowed_actors` when non-empty, or `string` when empty).
  - `execute_tool_call` reads `_actor` from top-level `arguments._actor` (`resources/read` reads `params._actor`), optionally unwraps nested `arguments.payload` (`invoke_*`) or `arguments.params` (`query_*`), and strips `_actor` before dispatching to `DomainRuntime` (which enforces fail-closed actor authorization).
  - Over Streamable HTTP (`POST`), explicit JSON `_actor` takes precedence over fallback HTTP headers extracted via `chromatofoor_http::actor_from_headers` (`X-Actor-Type` / `X-Actor`); `McpDomainServer::read_resource` over `stdio` dispatches with `actor: None`.
- **Transport Error & Response Semantics** (`src/lib.rs:McpDomainServer`, `src/lib.rs:handle_request_with_actor`, `src/lib.rs:routes_at`):
  - `McpDomainServer::call_tool` encodes `DomainError(ProblemDetails)` as pretty-printed RFC 9457 JSON inside `CallToolResult::error(...)` (`isError: true`), whereas `handle_request_with_actor` encodes `DomainError(ProblemDetails)` as JSON-RPC `-32603` with `error.data` populated by the structured `ProblemDetails` object (both return `-32602` / `invalid_params` for unknown tools, and `handle_request_with_actor` treats any `method.starts_with("notifications/")` as a notification returning `None`).
  - HTTP `POST` notifications (and notification-only batches) return `202 Accepted`; single requests whose `Accept` header includes `text/event-stream` without `application/json` return a single-frame SSE response (`event: message`).
- **Resource URI Parameter Resolution** (`src/lib.rs:read_esdm_resource`): `esdm://catalog` returns `runtime.catalog_json()` without actor checks; `esdm://{bc}/{query}[/{id}]` dispatches `{}` when `{id}` is absent, empty, or `"list"`, otherwise binds `{id}` to the first matching schema key in precedence order: `"id"` -> `*_id` -> first `required` property -> first property in `properties` -> `"id"`.

## Side Effects
- **Standard I/O** (`src/lib.rs:serve_mcp`, `src/lib.rs:McpStdio`): Reads and writes async JSON-RPC frames over `stdin` / `stdout` via `rmcp::transport::stdio_pipes()`.
- **Domain Store & Read-Model Dispatch** (`src/lib.rs:execute_tool_call`, `src/lib.rs:read_esdm_resource`): Appends events and mutates state in the backing store via `DomainRuntime::dispatch_command` on `invoke_*` tools, and reads projections via `DomainRuntime::dispatch_query` on `query_*` tools and `esdm://{bc}/{query}` resources.
- **HTTP & Live Event Streaming** (`src/lib.rs:routes_at`, feature `http`): Serves HTTP JSON/SSE responses on `POST {path}` and subscribes to `DomainRuntime::subscribe_events()` on `GET {path}` to stream live `notifications/domain_event` SSE frames (filtered by optional `subject_prefix` and `event_type` query parameters).

## Verification
```bash
cargo test -p chromatofoor-mcp --all-features
cargo clippy -p chromatofoor-mcp --all-targets --all-features -- -D warnings
```
