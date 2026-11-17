# Deterministic Code Generator (`chromatofoor-gen`)

* **Status:** Current
* **Last Updated:** 2026-11-16
* **Scope:** `chromatofoor-gen`, `esdm`

---

## 1. Overview

`chromatofoor-gen` compiles declarative ESDM specifications (`model.esdm.yaml` plus auto-discovered or explicit `model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `model.infra.esdm.yaml`) into a `#![deny(unsafe_code)]` multi-crate Rust Cargo workspace, isolated Given-When-Then integration tests (`tests/generated/main.rs`), presentation servers, and a dual-transport TypeScript client SDK (`typescript-client/`).

Its internal compiler modules mirror `chromatofoor` and `chromatofoor-derive` (`common`, `domain`, `bounded_context`, `integration`, `runtime`, `testing`, plus `presentation` and `graph`). See [`chromatofoor-gen/README.md`](../../chromatofoor-gen/README.md) for the complete ESDM `kind` and extension Support Matrix, and [`compiler-alignment-and-testbeds.md`](./compiler-alignment-and-testbeds.md) for the Red-phase stub and test-isolation specification.

---

## 2. Architecture & Design Decisions

### 2.1 Multi-Crate Workspace Layout, Inherited Lints, & Shared `domain` Crate
Each generated workspace (`<out-dir>/<domain-name>/`) isolates compilation units for fast incremental rebuilds and enforces workspace-wide safety and burndown lints (`[workspace.lints.rust] unsafe_code = "deny"` and `[workspace.lints.clippy] todo = "warn"`, inherited via `[lints] workspace = true` in every member crate):
- `domain/` — Shared lightweight crate declaring `#[chromatofoor::domain] pub struct <Domain>;` and `#[chromatofoor::external_system]` structs so bounded contexts and integration crates reference canonical problem-space types without circular dependencies.
- `bounded-contexts/<bc>/` (or `bounded-contexts/<subdomain>/<bc>/`, and `shared-kernels/<bc>/` when `tier: shared-kernel`) — Isolated crate per bounded context.
- `integration/policies/`, `integration/event-handlers/`, `integration/process-managers/<pm>/` — Cross-context consumer crates.
- `bootstrap/` — Declares `#[chromatofoor::subdomain]` and `#[chromatofoor::context_mapping]` relationships (which reference multiple bounded-context crates) and wires all bounded-context commands, read models, queries, and integration consumers into `DomainBuilder<Domain>` (`Domain::build` and `Domain::build_with_store`).
- `presentation/` — Declaratively scaffolded transport binaries (`api-server`, `graphql-server`, `mcp-server`, `interactive-tui`, `tauri-app`).
- `typescript-client/` — Generated `types.ts`, `ChromatofoorBridge` (`TauriBridge` + `WebBridge` + `MockBridge`), Svelte 5 Runes helpers, and typed `DomainClient`.

### 2.2 Non-Destructive Generated vs. Manual File Boundary & Red-Phase `todo!(...)` Stubs
Within every bounded-context and integration crate, `chromatofoor-gen` enforces a strict ownership boundary:
- **Generated (`write_generated_rust_file` / `write_rust_file`)**: Overwritten deterministically (formatted via `syn::parse_file` + `prettyplease::unparse` with a `// ⚠️ DO NOT EDIT ⚠️` header) on every run. Used exclusively for `src/generated/`, `domain/`, `bootstrap/`, `<crate>/tests/generated/**`, `typescript-client/`, and generated presentation entry points.
- **Manual (`write_manual_rust_file`)**: Scaffolded once with explicit `todo!("implement ...")` failure markers (without a `DO NOT EDIT` banner) and skipped on subsequent runs if the file already exists. Used for developer-owned logic in `src/domain/`, `src/application/read_models/`, `src/infrastructure/`, `integration/policies/src/handlers/`, `integration/event-handlers/src/handlers/`, and `integration/process-managers/*/src/process_manager.rs`.
- **Isolated Generated Test Directory (`sync_generated_test_dir`)**: Generated Given-When-Then suites live inside `<crate>/tests/generated/main.rs` (`aggregate_<name>.rs`, `dcb_<name>.rs`, `read_model_<name>.rs`, `process_manager_<name>.rs`). Re-running `chromatofoor-gen` cleanly purges obsolete generated test modules when YAML features change while leaving hand-written `<crate>/tests/*.rs` integration tests untouched.
- **AST-Synchronized Manual Glue (`ensure_manual_mod_file` & `sync_manual_pm_impls`)**: When new domain items or Process Manager reactions/timers are added to `model.esdm.yaml`, the generator parses existing manual `mod.rs` files with `syn::parse_file` to append only missing `pub mod` declarations (including `r#<kw>` escaping) and appends missing `has_ended`, `OnEvent<E>`, and `OnTimer<T>` impl blocks to existing `process_manager.rs` files without overwriting developer code or comments.

### 2.3 Faithful Schema Translation & Explicit Identifier Enforcement
- **No Silent Field Injection**: `chromatofoor-gen` emits only the fields declared in `model.esdm.yaml`. Missing command identifier fields on mutating/deleting commands are rejected upfront by `esdm::lint::validate_model` (and at Rust compile time by `validate_command_id_fields`), prompting the author to fix the ESDM model rather than hiding defects.
- **Typed Formats & Validation**: `common/schema.rs` maps JSON Schema `format: uuid` → `uuid::Uuid`, `format: identifier` → `chromatofoor::Identifier`, `date` / `date-time` → `chrono`, and translates length, numeric range, and nested object rules into `#[derive(chromatofoor::validator::Validate)]` + `#[validate(crate = "chromatofoor::validator")]`.
- **Event & Command Disambiguation**: Colliding short event/command names across bounded contexts are disambiguated deterministically when generating consumer enums, and multiple Process Manager reactions on the same event embed merged `(declared timers: set=[...], cancel=[...])` hints inside the initial `todo!(...)` literal.

### 2.4 Hydrating Given-When-Then Test Payloads via Serde JSON
- Generated BDD tests (`<crate>/tests/generated/*.rs`) serialize `given` / `when` / `then` YAML payloads into JSON literals hydrated via `serde_json::from_str(...)` and execute scenarios through `chromatofoor::testing::{given, given_process_manager, given_read_model}`, covering `then.events`, `events: []` (`then_nothing`), `then.rejection` (`then_rejected` / `then_rejected_reason`), `then.result` (`then_result_json`), `then.readModel` (`then_read_model_json`), and Process Manager command/timer/state/termination assertions.

### 2.5 Declarative Infrastructure & Presentation Bindings (`infrastructure/v1`)
- When `model.infra.esdm.yaml` (`--infra` or auto-discovered sibling) defines `kind: presentation-binding`, `chromatofoor-gen` scaffolds the requested presentation crates (`http-rest`, `graphql`, `mcp`, `cli`, `tauri-ipc`). Tauri scaffolding resolves `@chromatofoor/ui` paths dynamically and creates a minimal `dist/index.html` placeholder if absent so `cargo test --workspace --no-run` compiles without requiring a prior Node.js frontend build.

---

## 3. Determinism & Verification Invariants

1. Every collection derived from `HashMap` or `HashSet` MUST be sorted alphabetically prior to token generation.
2. Explicit and auto-discovered `model.lifecycle.esdm.yaml` and `model.feature.esdm.yaml` files fail closed on missing explicit paths or parse/deserialization errors before any files are written or purged.
3. Update `insta` snapshots when intentionally changing generator output (`RUSTC_WRAPPER="" cargo insta test -p chromatofoor-gen --accept`), regenerate affected workspaces, and verify zero git-diff non-determinism via `./scripts/verify_all.sh`.
