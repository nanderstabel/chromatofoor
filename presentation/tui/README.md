# Chromatofoor TUI Presentation Adapter (`chromatofoor-tui`)

## Overview
`chromatofoor-tui` provides an interactive Ratatui/Crossterm terminal user interface (TUI) for `chromatofoor::domain::DomainRuntime`. It dynamically introspects registered actors, actor-authorized commands and queries, and JSON Schema input templates to render a 42%/58% horizontal split-pane terminal application pairing wrapped modal command/query editors and styled result panes with a filterable, scrollable 1,000-event domain stream view.

## Interface
- **Presentation Adapter & Runner** (`src/lib.rs` — no Cargo feature flags):
  - `Tui` — Zero-sized adapter (`Debug`, `Default`, `Clone`, `Copy`, `pub const fn new() -> Self`) implementing `chromatofoor::runtime::Presentation` (`name() -> "tui"`, `async fn serve(self, runtime: Arc<DomainRuntime>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>`).
  - `async fn run(runtime: Arc<DomainRuntime>)` — Enters terminal raw mode, alternate screen, and bracketed paste on `stdout`, spawns a live event subscriber, ignores `KeyEventKind::Release` events, and drives an interactive modal loop:
    - Actor picker (`Up`/`Down`/`Enter`/`Esc`).
    - Main menu (`c` commands, `q` queries, `e`/`Tab` focus event stream, `a` switch actor, `f` filter events, `r` reset active filter, `x` clear log, `Esc` exit).
    - Wrapped multi-line command & query pickers (`Up`/`Down`/`Enter`/`Esc`, plus `f`/`r`/`x`).
    - Split-header `ratatui-textarea` payload/parameter editors submitting via `Ctrl+S` (`Esc` back).
    - Case-insensitive substring event-filter editor (`Enter` or `Ctrl+S` to apply, `Ctrl+U` to clear input, `Esc` to cancel).
    - Interactive event-stream focus (`Up`/`Down`, `j`/`k`, `PageUp`/`PageDown`, `Home`, `Enter` to inspect full event payload, `f`/`r`/`x`, `Esc`/`Tab` back).
    - Scrollable, word-wrapped dispatch result view (`[OK]` green border for successes including emitted event subjects/payloads vs. `[ERROR]` red border for failures; `Up`/`Down`, `j`/`k`, `PageUp`/`PageDown`, `Home` to scroll, `Esc`/`Enter` back).
  - `generate_template(schema: &serde_json::Value) -> String` — Expands a JSON Schema (`$ref`, single-element `allOf`, `anyOf`/`oneOf`, `default`, `enum`, `properties`, `items` emitting `[...]` for object items and `[]` for scalar/untyped arrays, and `uuid`/`date`/`date-time` string formats) into a pretty-printed JSON skeleton string (falling back to `"{}"`).

## Invariants
- **Safety & Lint Baseline** (`Cargo.toml`, `src/lib.rs`): Inherits workspace `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(panic, todo, unimplemented)` (with crate-level `#![allow(clippy::unwrap_used, clippy::expect_used)]` in `src/lib.rs`).
- **Actor-Scoped Menu Filtering & Dispatch** (`src/lib.rs:collect_actors`, `filter_commands`, `filter_queries`, `run`):
  - Falls back to `["anonymous"]` when `runtime.actors()` is empty so the actor list is always non-empty.
  - Populates command and query lists exclusively via `runtime.commands_for_actor(&current_actor)` and `runtime.queries_for_actor(&current_actor)`, and passes `Some(&current_actor)` on every dispatch.
- **Read-After-Write Synchronization** (`src/lib.rs:run`): Awaits best-effort `let _ = runtime.wait_caught_up().await` after successful `dispatch_command` before displaying the result screen.
- **Schema Property Depth Bound & Ring-Buffer Capacity** (`src/lib.rs:resolve_schema`, `generate_value`, `run`):
  - `generate_value` bounds nested `"properties"` and object `"items"` expansion at depth `> 8` (emitting `null`), whereas `resolve_schema` dereferences `$ref` / `allOf` / `anyOf` / `oneOf` chains without a depth counter (assumed acyclic).
  - The right-pane live event log (`VecDeque<EventEntry>`) prepends newest events (`push_front`), wraps subject/type/payload lines to pane width, filters visible rows case-insensitively across subject, event type, and payload JSON, and evicts entries past 1,000 items (`pop_back`).
- **Normal-Exit Terminal Restoration** (`src/lib.rs:run`): Restores raw mode, alternate screen, bracketed paste, and cursor visibility on normal `Esc` exit from the actor picker or main menu (no RAII `Drop` guard or panic hook runs on TTY `.unwrap()` panics or mid-`await` task cancellation, and the spawned event-subscriber task detaches until `DomainRuntime` drops).

## Side Effects
- **Terminal / TTY Control** (`src/lib.rs:run`): Mutates `std::io::stdout()` terminal state via Crossterm (`enable_raw_mode`, `EnterAlternateScreen`, `EnableBracketedPaste`, `LeaveAlternateScreen`, `DisableBracketedPaste`, `disable_raw_mode`), renders Ratatui frames, and polls input via `crossterm::event::poll(100ms)` / `read()`.
- **Background Event Subscription & Dispatch** (`src/lib.rs:run`): Subscribes to `DomainRuntime::subscribe_events()` on a detached `tokio::spawn` task, mutates store state via `DomainRuntime::dispatch_command` + `wait_caught_up`, and reads projections via `DomainRuntime::dispatch_query`.

## Verification
```bash
cargo test -p chromatofoor-tui --all-features
cargo clippy -p chromatofoor-tui --all-targets --all-features -- -D warnings
```
