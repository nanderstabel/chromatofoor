# Chromatofoor Code Generator (`chromatofoor-gen`)

## Overview
`chromatofoor-gen` is the deterministic compiler and domain-graph visualizer binary for the Chromatofoor Event Sourcing / CQRS / DDD platform (licensed under [GNU AGPL-3.0](./LICENSE) with the [Generated Code Output Exception](./OUTPUT_EXCEPTION.md)). It parses declarative `ESDM` YAML specifications (`model.esdm.yaml` alongside `.lifecycle.esdm.yaml`, `.feature.esdm.yaml`, and `.infra.esdm.yaml` extensions), validates cross-document semantics via `esdm::lint`, and emits `#![deny(unsafe_code)]` multi-crate Rust Cargo workspaces targeting `#[chromatofoor::<kind>]` macros, explicit `todo!(...)` Red-phase stubs, isolated `tests/generated/main.rs` BDD suites, composable presentation binaries, and dual-transport TypeScript/Svelte 5 SDKs.

## Interface
- **Binary CLI & Subcommands** (`src/main.rs`, `src/runtime/pipeline.rs`):
  - `chromatofoor-gen --input <PathBuf> --out-dir <PathBuf> [OPTIONS]` — Compiles ESDM specifications into `<out-dir>/<domain-name>/`:
    - `-i, --input <PathBuf>` — Primary `model.esdm.yaml` specification (required in codegen mode).
    - `-f, --features <PathBuf>` — Optional `given-when-then/v1` BDD scenarios file (auto-discovers sibling `model.feature.esdm.yaml` when omitted; fails closed on missing explicit path or malformed YAML).
    - `--lifecycle <PathBuf>` — Optional `lifecycle/v1` file (auto-discovers sibling `model.lifecycle.esdm.yaml` when omitted; fails closed on missing explicit path or malformed YAML).
    - `--infra <PathBuf>` — Optional `infrastructure/v1` file (auto-discovers sibling `model.infra.esdm.yaml` when omitted).
    - `-o, --out-dir <PathBuf>` — Target workspace parent directory (required in codegen mode).
    - `--chromatofoor-path <String>` — Relative or absolute Cargo dependency path to the `chromatofoor` runtime crate (default `"../../chromatofoor"`).
    - `--emit-graph <PathBuf>` — Exports the resolved domain graph (`domain`, `nodes`, `links`) as pretty-printed JSON via `graph::emit_graph_file`.
  - `chromatofoor-gen visualize -i, --input <PathBuf>` — Serves the interactive local HTTP domain-graph visualizer (`src/graph/visualizer.html` + `GET /graph.json`) for a serialized graph JSON file.
- **Cargo Feature Flags** (`Cargo.toml` — binary-only package, no `[lib]` target):
  - `default = ["tui"]`
  - `full = ["tui", "http", "graphql", "mcp", "tauri", "typescript"]`
  - Individual transport/client flags: `tui`, `http`, `graphql`, `mcp`, `tauri` (`["typescript"]`), `typescript`.
- **Internal Compiler Modules** (`src/` — aligned with `chromatofoor` and `chromatofoor-derive`):
  - `common` (`src/common/{context.rs, writer.rs, schema.rs}`) — `GeneratorContext`, `BoundedContextGenerator`, `RoutingContext`, `resolve_bc_dir` (routing `tier: "shared-kernel"` to `shared-kernels/<bc>/`, subdomains to `bounded-contexts/<subdomain>/<bc>/`, and default BCs to `bounded-contexts/<bc>/`), `MEMBER_LINTS_TOML`, `write_generated_rust_file` (`write_rust_file`), `write_manual_rust_file`, `ensure_manual_mod_file`, `map_type`, `generate_struct_fields`, and `generate_type_for_prop`.
  - `domain` (`src/domain/mod.rs`) — `generate_domain_crate` emitting the shared `<workspace>/domain` crate (`#[chromatofoor::domain]` and `#[chromatofoor::external_system]`).
  - `bounded_context` (`src/bounded_context/{crate_scaffold.rs, consistency.rs, application.rs, domain_service.rs, prelude.rs}`) — Per-BC `Cargo.toml` & `src/infrastructure/` scaffolding (`generate_bounded_context_scaffolds`), consistency units (`generate_consistency_units`, `generate_actors`, `generate_schema_structs`), read models & queries (`generate_application`), domain services (`generate_domain_services`), and BC preludes & `register` functions (`generate_preludes`).
  - `integration` (`src/integration/{handler_common.rs, policy.rs, event_handler.rs, process_manager.rs}`) — `generate_policies`, `generate_event_handlers`, and `generate_process_managers` (`ResolvedPmSpec`, `sync_manual_pm_impls`).
  - `runtime` (`src/runtime/{pipeline.rs, workspace.rs, bootstrap.rs}`) — Multi-file ESDM ingestion & validation (`run_codegen`), root `<workspace>/Cargo.toml` & workspace orchestration (`generate_workspace`), and `bootstrap` crate composition (`generate_bootstrap_crate` with `#[chromatofoor::subdomain]`, `#[chromatofoor::context_mapping]`, and `Domain::{build, build_with_store}`).
  - `testing` (`src/testing/{mod.rs, bounded_context.rs, process_manager.rs}`) — `sync_generated_test_dir`, `generate_tests`, and `generate_process_manager_tests` emitting isolated `<crate>/tests/generated/main.rs` (`aggregate_<name>.rs`, `dcb_<name>.rs`, `read_model_<name>.rs`, `process_manager_<name>.rs`) suites.
  - `presentation` (`src/presentation/{transports.rs, typescript.rs}`) — `ActiveTransports`, `generate_transports` (unified HTTP `api-server` composing `http` + `graphql` + `mcp` HTTP protocols, standalone `mcp-server` over stdio, `interactive-tui`, and `tauri-app`), and `#[cfg(feature = "typescript")] generate_typescript` (`typescript-client/`).
  - `graph` (`src/graph/{mod.rs, visualizer.html}`) — `emit_graph_file` and `run_visualizer_server`.

## Invariants
- **Safety & Lint Baseline** (`Cargo.toml`, `src/main.rs`, `src/common/writer.rs`, `src/runtime/workspace.rs`):
  - `chromatofoor-gen` itself inherits workspace `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(panic, todo, unimplemented)` (with crate-local `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_arguments, clippy::ptr_arg)]` in `src/main.rs`).
  - Every generated workspace root `Cargo.toml` declares `[workspace.lints.rust] unsafe_code = "deny"` and `[workspace.lints.clippy] todo = "warn"`, and every generated member crate (`domain`, bounded contexts, shared kernels, `bootstrap`, `policies`, `event-handlers`, `process-managers/*`, and all 5 presentation binaries) declares `[lints]\nworkspace = true`.
- **Fail-Closed Validation Gates** (`src/runtime/pipeline.rs`, `src/bounded_context/{consistency.rs, application.rs}`):
  - Pre-flight in `runtime/pipeline.rs`: requires at least one `kind: domain` document, fails closed on missing explicit `--lifecycle` / `--features` files, malformed YAML or `EsdmDoc` deserialization errors in lifecycle or feature files, or duplicate `aggregate-lifecycle` specs, and aborts before writing or purging any files if `esdm::lint::validate_model` reports any violations.
  - Emission-time actor checks: `generate_consistency_units` and `generate_application` abort via `anyhow::bail!` if any command or query `actors` entry is undeclared on its bounded context.
- **Explicit Red-Phase `todo!(...)` Scaffolds** (`src/bounded_context/*`, `src/integration/*`): Newly created manual trait implementations (`Command::decide`, `Event::evolve`, `Project::project`, `<Service>Service::<fn>`, `React::react`, `Handle::handle`, `OnEvent::on_event`, `OnTimer::on_timer`, and `ProcessManager::has_ended` when `endsWhen` is non-empty) emit trait/type-qualified `todo!("implement ...")` bodies that compile cleanly under `-D warnings -A clippy::todo`, panic deterministically in Red-phase `cargo test`, and surface in `cargo clippy` (`-D clippy::todo` fails CI until implemented).
- **AST Formatting & Canonical Macro Parity** (`src/common/writer.rs`, `src/bounded_context/consistency.rs`): Token-stream modules written via `write_generated_rust_file` / `write_manual_rust_file` are validated with `syn::parse_file` and formatted via `prettyplease::unparse` (complemented by a workspace-wide `cargo fmt` pass), emitting the exact same `#[chromatofoor::<kind>]` attributes and trait signatures used by hand-authored Rust domains.
- **100% Deterministic Output Ordering** (`src/common/schema.rs`, `src/bounded_context/*`, `src/integration/*`, `src/runtime/*`): Domain struct properties, enum variants (first sorted variant marked `#[default]`), workspace `Cargo.toml` members, bounded contexts, subdomains, directional asymmetric `context-mapping` dependencies, external systems, process managers, timers, and `tests/generated/main.rs` modules are sorted lexicographically, while `IndexMap`-backed read-model, query, and process-manager fields and BDD scenario lists preserve YAML declaration order so repeated runs produce zero `git diff`.
- **Non-Destructive Manual Code & Test Boundaries** (`src/common/writer.rs`, `src/integration/process_manager.rs`, `src/testing/mod.rs`):
  - Overwrites generated scaffolding (`src/generated/**`, `domain/`, `bootstrap/`, `<crate>/tests/generated/**`, presentation `main.rs` entry points, and `typescript-client/`), purging stale generated per-unit test files cleanly when YAML features are renamed or removed.
  - Never overwrites existing manual domain/infrastructure/test files (`<bc>/src/domain/**`, `<bc>/src/application/read_models/**`, `<bc>/src/infrastructure/<ext>.rs`, `integration/{policies,event-handlers}/src/handlers/**`, `presentation/tauri-app/src/App.svelte`, and hand-written `<crate>/tests/*.rs` files lacking the `// ⚠️ DO NOT EDIT ⚠️` header).
  - Parses existing `mod.rs` (`ensure_manual_mod_file`) and `src/process_manager.rs` (`sync_manual_pm_impls`) via `syn::parse_file` to append only newly introduced `pub mod` items or missing `has_ended` / `OnEvent<E>` / `OnTimer<T>` impl blocks.
- **Declarative Transport Precedence** (`src/runtime/pipeline.rs`): When `kind: presentation-binding` documents exist in `.infra.esdm.yaml`, `ActiveTransports` is derived solely from `doc.transport` (`http` | `http-rest`, `cli`, `graphql`, `tauri-ipc`, `mcp`), overriding Cargo transport feature defaults (`typescript` generation remains gated by `#[cfg(feature = "typescript")]`).

## Side Effects
- **Filesystem Reads & Writes** (`src/runtime/pipeline.rs`, `src/presentation/transports.rs`, `src/testing/mod.rs`): Reads input `.esdm.yaml` documents (and optional `branding/logo-*.jpg` assets when scaffolding `tauri-app`), creates/updates `<out-dir>/<domain-name>/` directory trees (including placeholder `presentation/tauri-app/dist/index.html` for `tauri::generate_context!()`), synchronizes `<crate>/tests/generated/`, removes stale `presentation/tauri-app/vite.config.js` if present, and writes optional `--emit-graph` JSON files.
- **Subprocess Formatting** (`src/runtime/pipeline.rs`): Spawns `cargo fmt` with `current_dir` set to `<out-dir>/<domain-name>/` after workspace emission (logging a warning if `cargo fmt` cannot be spawned).
- **Local HTTP Visualizer Socket** (`src/graph/mod.rs:run_visualizer_server`): When invoked with `chromatofoor-gen visualize`, binds `std::net::TcpListener` on `127.0.0.1:0` and serves embedded HTML + `/graph.json` over HTTP/1.1.

## Generator Support Matrix

### Core ESDM Schema Kinds (`schemas/core/v1.yaml`)

| Schema `kind`                  | Supported? | Code Generation Status / Notes                                                                                                                                                              |
| :----------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `domain`                       |     ✅     | Generates the Cargo workspace, shared `domain` crate (`#[chromatofoor::domain]`), and `bootstrap` crate (`Domain::build_with_store` returning `DomainRuntime`).                             |
| `subdomain`                    |     ✅     | Emits `#[chromatofoor::subdomain]` declarations in `bootstrap` and routes bounded contexts into `bounded-contexts/<subdomain>/<bc>/` (unless labeled `tier: "shared-kernel"`).              |
| `bounded-context`              |     ✅     | Generates an isolated library crate (`shared-kernels/<bc>/` or `bounded-contexts/[<subdomain>/]<bc>/`) using `#[chromatofoor::bounded_context(scope = <Domain>)]`, with `generated/`, `domain/`, and conditional `application/read_models/` & `infrastructure/`. |
| `aggregate`                    |     ✅     | Generates `#[chromatofoor::aggregate]` state structs/enums (`architecturekit::State`), `identified_by` metadata, and typed domain errors (`#[error("{name}: {rule}")]` + RFC 9457 `Problem`). |
| `command`                      |     ✅     | Generates `#[chromatofoor::command]` enums and payload structs with `validator` constraints, `creates` lifecycle flags, and manual `Command::decide` `todo!(...)` stubs.                    |
| `event`                        |     ✅     | Generates `#[chromatofoor::event]` enums and payload structs (`architecturekit::Event`/`Events`) and manual `Event::evolve` `todo!(...)` stubs.                                             |
| `dynamic-consistency-boundary` |     ✅     | Generates `#[chromatofoor::dynamic_consistency_boundary]` units that fold `consults` events via `Event<Dcb>` directly from `architecturekit::Store`.                                        |
| `read-model`                   |     ✅     | Generates `#[chromatofoor::read_model]` projections backed by `DocumentView<Record>` (`architecturekit::InMemoryView`) and manual `Project<E>` `todo!(...)` stubs.                          |
| `query`                        |     ✅     | Generates `#[chromatofoor::query]` structs and `Query<ReadModel>` implementations (keyed `records.get`, collection `records.all`, or singleton `records.latest`) registered on `DomainBuilder`. |
| `entity`                       |     ✅     | Generates `#[chromatofoor::entity]` structs with `identified_by` (`schema` / `static`) and `validator` constraints.                                                                         |
| `value-object`                 |     ✅     | Generates `#[chromatofoor::value_object]` structs with schema validation constraints.                                                                                                       |
| `domain-service`               |     ✅     | Generates `#[chromatofoor::domain_service]` async traits, request/response DTOs, error enums, and manual `todo!(...)` implementation stubs.                                                 |
| `actor`                        |     ✅     | Generates `#[chromatofoor::actor]` unit structs and wires fail-closed `Authorized` actor lists across commands and queries.                                                                 |
| `policy`                       |     ✅     | Generates an `integration/policies` crate using `#[chromatofoor::policy]` and manual `React<E>` `todo!(...)` implementations returning typed `<Policy>Commands`.                            |
| `event-handler`                |     ✅     | Generates an `integration/event-handlers` crate using `#[chromatofoor::event_handler]` and manual `Handle<E>` `todo!(...)` implementations for outbound side effects.                       |
| `process-manager`              |     ✅     | Generates `integration/process-managers/<pm>` crates using `#[chromatofoor::process_manager]` (`OnEvent`, `OnTimer`) with `ProcessTimer` scheduling and store-backed state persistence.     |
| `external-system`              |     ✅     | Emits `#[chromatofoor::external_system]` declarations in `domain` and scaffolds async port traits in `src/infrastructure/`.                                                                 |
| `context-mapping`              |     ✅     | Emits `#[chromatofoor::context_mapping]` relationships in `bootstrap` and resolves directional asymmetric `Cargo.toml` dependencies + external-system infrastructure stubs.                 |

### Extension Schemas & Capabilities

| Extension / Feature                                         | Supported? | Code Generation Status / Notes                                                                                                                           |
| :---------------------------------------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aggregate-lifecycle` (`schemas/lifecycle/v1.yaml`)         |     ✅     | Validates non-empty `created` commands per stateful aggregate and emits `creates` on creation `#[chromatofoor::command]` structs for stream preconditions. |
| `feature` BDD Scenarios (`schemas/given-when-then/v1.yaml`) |     ✅     | Generates isolated `tests/generated/main.rs` suites (`then.events`, `then.rejection`, `then.result`, `then.readModel`, PM assertions) via `chromatofoor::testing`. |
| `presentation-binding` (`schemas/infrastructure/v1.yaml`)   |     ✅     | Selects active transport crates (`api-server`, `interactive-tui`, `graphql-server`, `mcp-server`, `tauri-app`) from `doc.transport`.                     |
| JSON Schema Types & Formats                                 |     ✅     | Maps `uuid` → `uuid::Uuid`, `identifier` → `chromatofoor::Identifier`, `date` / `date-time` → `chrono`, nested objects, arrays, and string `enum`s.      |
| JSON Schema Validation                                      |     ✅     | Emits `validator::Validate` attributes (`length`, `range`, and `nested` validation) enforced before `Command::decide`.                                   |
| Dual-Transport TypeScript SDK (`typescript-client/`)        |     ✅     | Generates typed `types.ts`, `ChromatofoorBridge` (`WebBridge` + `TauriBridge` + `MockBridge`), `DomainClient`, and Svelte 5 Runes state factories.       |

## Verification
```bash
cargo test -p chromatofoor-gen --all-features
cargo clippy -p chromatofoor-gen --all-targets --all-features -- -D warnings
```
