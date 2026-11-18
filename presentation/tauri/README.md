# Chromatofoor Tauri Presentation Adapter (`chromatofoor-tauri`)

## Overview
`chromatofoor-tauri` provides Tauri v2 (`tauri = "2.11.5"`) desktop and mobile (`android`, `ios`) IPC bindings for `chromatofoor::domain::DomainRuntime`. It wraps `Arc<DomainRuntime>` in managed `DomainState` and exposes seven `#[tauri::command]` handlers in `chromatofoor_tauri::commands` for command dispatch, read-model querying, domain catalog introspection, pull-based event-store snapshot retrieval, and native window controls (consumed by `TauriBridge` in generated TypeScript SDKs).

## Interface
- **Managed Runtime State** (`src/lib.rs` — no Cargo feature flags):
  - `DomainState { pub dispatcher: Arc<DomainRuntime> }` — Tauri managed state container (`pub const fn new(dispatcher: Arc<DomainRuntime>) -> Self`).
- **Tauri IPC Commands** (`src/lib.rs`, `pub mod commands` — registered directly via `tauri::generate_handler!`; Tauri v2 deserializes multi-word JS parameter keys in `camelCase` such as `cmdName`, `queryName`, `commandName`):
  - `async fn dispatch_command(ctx: String, agg: String, cmd_name: String, payload: Value, metadata: HashMap<String, String>, state: State<'_, DomainState>) -> Result<Value, ProblemDetails>` — Dispatches `(ctx, agg, cmd_name)`, performs a best-effort `wait_caught_up()`, and returns `{"events": [...]}` (or structured RFC 9457 `ProblemDetails` on failure).
  - `async fn query_read_model(ctx: String, query_name: String, params: Option<Value>, metadata: Option<HashMap<String, String>>, state: State<'_, DomainState>) -> Result<Value, ProblemDetails>` — Dispatches `(ctx, query_name)` with `params` defaulting to `{}`.
  - `async fn execute_command(command_name: String, payload: Value, metadata: Option<HashMap<String, String>>, state: State<'_, DomainState>) -> Result<Value, ProblemDetails>` — Dispatches dot-delimited `"ctx.agg.cmd"` or `"ctx.cmd"`, performs a best-effort `wait_caught_up()`, and returns `{"events": [...]}`.
  - `async fn execute_query(query_name: String, params: Value, metadata: Option<HashMap<String, String>>, state: State<'_, DomainState>) -> Result<Value, ProblemDetails>` — Dispatches dot-delimited `"ctx.query"`.
  - `async fn get_schema(state: State<'_, DomainState>) -> Result<Value, String>` — Returns `state.dispatcher.catalog_json()`.
  - `async fn get_events(state: State<'_, DomainState>) -> Result<Vec<Value>, String>` — Collects and returns a caught-up snapshot of all domain events under `domain_topic()` (propagating catch-up errors).
  - `fn window_control(window: WebviewWindow, action: String) -> Result<bool, String>` — Executes `"drag"`, `"minimize"`, `"toggle_maximize"`, or `"close"`.

## Invariants
- **Strict Rust Safety & Lint Baseline** (`Cargo.toml`): Inherits workspace lints enforcing `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)` (`Presentation` is not implemented because Tauri owns the desktop event loop).
- **Desktop Actor Resolution Fallback** (`src/lib.rs:commands::resolve_command_actor`, `resolve_query_actor`): Resolves the caller actor in precedence order:
  1. Explicit `metadata.get("actor_type").or_else(|| metadata.get("actor"))`, filtered by `!s.is_empty()` (if `"actor_type"` is present as `""`, it short-circuits `"actor"` and falls through to Step 2).
  2. First allowed actor declared on the target `CommandDescriptor` / `QueryDescriptor`.
  3. First actor registered on `DomainRuntime::actors()`.
  4. `None` (subject to `DomainRuntime`'s fail-closed authorization check).
- **Dot-Delimited Routing Contracts** (`src/lib.rs:commands::execute_command`, `execute_query`):
  - `execute_command` requires either 3 segments (`"ctx.target.cmd"`) or 2 segments (`"ctx.cmd"`, inferring `target` from the first matching command in `ctx` or falling back to `ctx`); any other arity returns `Err`.
  - `execute_query` requires exactly 2 segments (`"ctx.query"`).
- **Event Envelope & Window Action Shapes** (`src/lib.rs:commands::serialize_event`, `window_control`):
  - Serialized events always include `id`, `subject`, `instance_id` (final `'/'` segment of `subject`), `type`, `event_name` (final `'/'` segment of `type`), `source`, `time` (RFC 3339), and `data`.
  - `window_control` returns `true` when `toggle_maximize` maximizes an unmaximized window, `false` for all other valid actions, and `Err` on unknown action names.

## Side Effects
- **Store Mutations & Catch-Up Synchronization** (`src/lib.rs:commands::dispatch_command`, `execute_command`): Appends events via `DomainRuntime::dispatch_command` and awaits `let _ = DomainRuntime::wait_caught_up().await` before returning over IPC.
- **Read-Model Queries & Transient Event Snapshots** (`src/lib.rs:commands::query_read_model`, `execute_query`, `get_events`): Reads projections via `DomainRuntime::dispatch_query`; `get_events` runs a short-lived `Store::start_projection` collector (`"{topic}-tauri-events"`), awaits `architecturekit::wait_caught_up` (propagating errors), and cancels the projection token (no persistent background push emitter is spawned).
- **Native Window Mutations** (`src/lib.rs:commands::window_control`): Invokes synchronous OS/webview window operations (`start_dragging`, `minimize`, `is_maximized`, `unmaximize`, `maximize` on desktop targets via `#[cfg(not(any(target_os = "android", target_os = "ios")))]`, and `close` across all platforms) on `tauri::WebviewWindow`.

## Verification
```bash
cargo test -p chromatofoor-tauri --all-features
cargo clippy -p chromatofoor-tauri --all-targets --all-features -- -D warnings
```
