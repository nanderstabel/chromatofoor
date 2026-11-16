# Deterministic Code Generator (`chromatofoor-gen`)

* **Status:** Current
* **Last Updated:** 2026-11-14
* **Scope:** `chromatofoor-gen`, `esdm`

---

## 1. Overview

`chromatofoor-gen` compiles declarative ESDM specifications (`model.esdm.yaml` plus optional `model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `model.infra.esdm.yaml`) into a multi-crate Rust Cargo workspace, Given-When-Then integration tests, presentation servers, and a dual-transport TypeScript client SDK (`typescript-client/`).

See [`chromatofoor-gen/README.md`](../../chromatofoor-gen/README.md) for the complete ESDM `kind` and extension Support Matrix.

---

## 2. Architecture & Design Decisions

### 2.1 Multi-Crate Workspace Layout & Shared `domain` Crate
Each generated workspace (`<out-dir>/<domain-name>/`) isolates compilation units for fast incremental rebuilds:
- `domain/` — Shared lightweight crate declaring `#[chromatofoor::domain] pub struct <Domain>;`, `#[chromatofoor::subdomain]`, `#[chromatofoor::external_system]`, and `#[chromatofoor::context_mapping]` so bounded contexts and integration crates reference the canonical domain type without circular dependencies.
- `bounded-contexts/<bc>/` (or `bounded-contexts/<subdomain>/<bc>/`, and `shared-kernels/<bc>/` when `tier: shared-kernel`) — Isolated crate per bounded context.
- `integration/policies/`, `integration/event-handlers/`, `integration/process-managers/<pm>/` — Cross-context consumer crates.
- `bootstrap/` — Wires all bounded-context commands, read models, queries, and integration consumers into `DomainBuilder<Domain>` (`Domain::build` and `Domain::build_with_store`).
- `presentation/` — Declaratively scaffolded transport binaries (`api-server`, `graphql-server`, `mcp-server`, `interactive-tui`, `tauri-app`).
- `typescript-client/` — Generated `types.ts`, `ChromatofoorBridge` (`TauriBridge` + `WebBridge`), and typed `DomainClient`.

### 2.2 Non-Destructive Generated vs. Manual File Boundary
Within every bounded-context and integration crate, `chromatofoor-gen` enforces a strict ownership boundary:
- **Generated (`write_generated_rust_file` / `write_rust_file`)**: Overwritten deterministically (formatted via `syn::parse_file` + `prettyplease::unparse` with a `// ⚠️ DO NOT EDIT ⚠️` header) on every run. Used exclusively for `src/generated/`, `domain/`, `bootstrap/`, `tests/*_tests.rs`, `typescript-client/`, and generated presentation entry points.
- **Manual (`write_manual_rust_file`)**: Scaffolded once without a `DO NOT EDIT` banner and skipped on subsequent runs if the file already exists. Used for developer-owned logic in `src/domain/`, `src/application/read_models/`, `src/infrastructure/`, `integration/policies/src/policies/`, `integration/event-handlers/src/handlers/`, and `integration/process-managers/*/src/process_manager.rs`.
- **AST-Synchronized Manual Glue (`ensure_manual_mod_file` & `sync_manual_pm_methods`)**: When new domain items or Process Manager reactions are added to `model.esdm.yaml`, the generator parses existing manual `mod.rs` files with `syn::parse_file` to append only missing `pub mod` declarations (including `r#<kw>` escaping) and appends missing `on_<reaction>` method stubs to existing `process_manager.rs` `impl` blocks without overwriting existing code.

### 2.3 Faithful Schema Translation & Explicit Identifier Enforcement
- **No Silent Field Injection**: `chromatofoor-gen` emits only the fields declared in `model.esdm.yaml`. Missing command identifier fields on mutating/deleting commands are rejected upfront by `esdm::lint::validate_model` (and at Rust compile time by `validate_command_id_fields`), prompting the author to fix the ESDM model rather than hiding defects.
- **Typed Formats & Validation**: `schema_utils.rs` maps JSON Schema `format: uuid` → `uuid::Uuid`, `format: identifier` → `chromatofoor::Identifier`, `date` / `date-time` → `chrono`, and translates length, numeric range, regex pattern, and nested object rules into `#[derive(chromatofoor::validator::Validate)]` + `#[validate(crate = "chromatofoor::validator")]`.
- **Event & Command Disambiguation**: Colliding short event/command names across bounded contexts are disambiguated deterministically when generating consumer enums, and multiple Process Manager reactions on the same event are merged into a single handler dispatch arm.

### 2.4 Hydrating Given-When-Then Test Payloads via Serde JSON
- Generated BDD tests (`tests/*_tests.rs`) serialize `given` / `when` / `then` YAML payloads into JSON literals hydrated via `serde_json::from_str(...)` and execute scenarios through `chromatofoor::testing::{given, given_process_manager, given_read_model}`.

### 2.5 Declarative Infrastructure & Presentation Bindings (`infrastructure/v1`)
- When `model.infra.esdm.yaml` (`--infra` or auto-discovered sibling) defines `kind: presentation-binding`, `chromatofoor-gen` scaffolds the requested presentation crates (`http-rest`, `graphql`, `mcp`, `cli`, `tauri-ipc`) and wires custom specification HTTP routes (paths, methods, status codes) into `chromatofoor-http`. Tauri scaffolding targets desktop configurations by default (deferring heavy Android/iOS SDK directories unless initialized explicitly).

---

## 3. Determinism & Verification Invariants

1. Every collection derived from `HashMap` or `HashSet` MUST be sorted alphabetically prior to token generation.
2. Explicit `--lifecycle` paths fail closed if missing, unparseable, or containing duplicate aggregate scopes.
3. Update `insta` snapshots when intentionally changing generator output (`RUSTC_WRAPPER="" cargo insta test -p chromatofoor-gen --accept`), regenerate affected workspaces, and verify zero git-diff non-determinism via `./scripts/verify_all.sh`.
