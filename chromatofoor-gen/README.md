# Chromatofoor Code Generator (`chromatofoor-gen`)

## Overview
`chromatofoor-gen` is the deterministic compiler and domain-graph visualizer binary for the Chromatofoor Event Sourcing / CQRS / DDD platform (licensed under [GNU AGPL-3.0](./LICENSE) with the [Generated Code Output Exception](./OUTPUT_EXCEPTION.md)). It parses declarative `ESDM` YAML specifications (`model.esdm.yaml` alongside `.lifecycle.esdm.yaml`, `.feature.esdm.yaml`, and `.infra.esdm.yaml` extensions), validates cross-document semantics via `esdm::lint`, and emits multi-crate Rust Cargo workspaces targeting `#[chromatofoor::<kind>]` macros, `#[tokio::test]` BDD suites, composable presentation binaries, and dual-transport TypeScript/Svelte 5 SDKs.

## Interface
- **Binary CLI & Subcommands** (`src/main.rs`):
  - `chromatofoor-gen --input <PathBuf> --out-dir <PathBuf> [OPTIONS]` — Compiles ESDM specifications into `<out-dir>/<domain-name>/`:
    - `-i, --input <PathBuf>` — Primary `model.esdm.yaml` specification (required in codegen mode).
    - `-f, --features <PathBuf>` — Optional `given-when-then/v1` BDD scenarios file.
    - `--lifecycle <PathBuf>` — Optional `lifecycle/v1` file (auto-discovers sibling `model.lifecycle.esdm.yaml` when omitted).
    - `--infra <PathBuf>` — Optional `infrastructure/v1` file (auto-discovers sibling `model.infra.esdm.yaml` when omitted).
    - `-o, --out-dir <PathBuf>` — Target workspace parent directory (required in codegen mode).
    - `--chromatofoor-path <String>` — Relative Cargo dependency path to the `chromatofoor` runtime crate (default `"../../chromatofoor"`).
    - `--emit-graph <PathBuf>` — Exports the resolved domain graph (`domain`, `nodes`, `links`) as pretty-printed JSON via `generator::graph::emit_graph_file`.
  - `chromatofoor-gen visualize -i, --input <PathBuf>` — Serves the interactive local HTTP domain-graph visualizer (`generator/visualizer.html` + `GET /graph.json`) for a serialized graph JSON file.
- **Cargo Feature Flags** (`Cargo.toml` — binary-only package, no `[lib]` target):
  - `default = ["tui"]`
  - `full = ["tui", "http", "rest", "graphql", "mcp", "tauri", "typescript"]`
  - Individual transport/client flags: `tui`, `http`, `rest` (`["http"]`), `graphql`, `mcp`, `tauri` (`["typescript"]`), `typescript`.
- **Internal Compiler Modules** (`src/`):
  - `model` (`src/model.rs`) — Re-exports `esdm::*`.
  - `schema_utils` (`src/schema_utils.rs`) — `map_type`, `generate_struct_fields`, and `generate_type_for_prop` translating JSON Schema formats and bounds into `validator::Validate` + `architecturekit::Schema` Rust types.
  - `generator` (`src/generator/mod.rs`) — `GeneratorContext`, `BoundedContextGenerator`, `RoutingContext`, `resolve_bc_dir` (routing `tier: "shared-kernel"` to `shared-kernels/<bc>/`, subdomains to `bounded-contexts/<subdomain>/<bc>/`, and default BCs to `bounded-contexts/<bc>/`), `make_doc_comment`, `format_delivery_and_idempotency_tokens`, `format_scope_path`, `get_relative_cargo_path`, `write_generated_rust_file` (`write_rust_file`), `write_manual_rust_file`, and `ensure_manual_mod_file`.
  - `generator::workspace` & `generator::bootstrap` — `ActiveTransports`, `generate_workspace` (root `Cargo.toml`, shared `domain` crate, bounded-context crates, and presentation binaries `api-server`, `graphql-server`, `mcp-server`, `interactive-tui`, `tauri-app`), and `generate_bootstrap_crate` (`bootstrap::Domain::{build, build_with_store}`).
  - `generator::{domain, application, domain_service, prelude}` — `generate_domain`, `generate_actors`, `generate_schema_structs`, `generate_application`, `generate_domain_services`, and `generate_preludes`.
  - `generator::{policies, event_handlers, process_manager}` — `generate_policies`, `generate_event_handlers`, and `generate_process_managers`.
  - `generator::{tests, process_manager_tests}` — `generate_tests` and `generate_process_manager_tests` emitting `#[tokio::test]` BDD suites.
  - `generator::typescript` (`#[cfg(feature = "typescript")]`) — `generate_typescript` emitting `typescript-client/` (`types.ts`, `bridge.ts`, `client.ts`, `domain.svelte.ts`, `index.ts`).
  - `generator::graph` — `emit_graph_file` domain graph JSON serializer (`customer-supplier` context-mapping links, events, commands, projections, and reactions).

## Invariants
- **Safety & Lint Baseline** (`Cargo.toml`, `src/main.rs`): Inherits workspace `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(panic, todo, unimplemented)` (with crate-local `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_arguments, clippy::ptr_arg)]` in `src/main.rs`).
- **Validation Gates** (`src/main.rs`, `src/generator/domain.rs`, `src/generator/application.rs`):
  - Pre-flight in `src/main.rs`: requires at least one `kind: domain` document, fails fast on missing explicit `--lifecycle` files, malformed lifecycle YAML, or duplicate `aggregate-lifecycle` specs (whereas `--input`, `--features`, and `--infra` warn and skip unparseable documents or missing optional paths), and aborts before writing any files if `esdm::lint::validate_model` reports any violations.
  - Emission-time actor checks: `generate_domain` and `generate_application` abort via `anyhow::bail!` if any command or query `actors` entry is undeclared on its bounded context.
- **AST Formatting & Canonical Macro Parity** (`src/generator/mod.rs`, `src/generator/domain.rs`): Token-stream modules written via `write_generated_rust_file` / `write_manual_rust_file` are validated with `syn::parse_file` and formatted via `prettyplease::unparse` (complemented by a workspace-wide `cargo fmt` pass for template/entrypoint files), emitting the exact same `#[chromatofoor::<kind>]` attributes and trait signatures used by hand-authored Rust domains.
- **100% Deterministic Output Ordering** (`src/schema_utils.rs`, `src/generator/*`): Domain struct properties, enum variants (first sorted variant marked `#[default]`), workspace `Cargo.toml` members, bounded contexts, subdomains, directional asymmetric `context-mapping` dependencies, external systems, process managers, and timers (`BTreeMap`/`BTreeSet`) are sorted lexicographically, while `IndexMap`-backed read-model, query, and process-manager fields and BDD scenario lists preserve exact YAML declaration order so repeated runs produce zero `git diff`.
- **Non-Destructive Manual Code Boundaries** (`src/generator/mod.rs`, `src/generator/process_manager.rs`, `src/generator/workspace.rs`):
  - Overwrites generated scaffolding (`src/generated/**`, `domain/`, `bootstrap/`, `tests/`, presentation `main.rs` entry points, and `typescript-client/`).
  - Never overwrites existing manual domain/infrastructure files (`write_manual_rust_file` skips existing `<bc>/src/domain/**`, `<bc>/src/application/read_models/**`, `<bc>/src/infrastructure/<ext>.rs`, `integration/{policies,event-handlers}/src/handlers/**`, and `presentation/tauri-app/src/App.svelte`).
  - Parses existing `mod.rs` (`ensure_manual_mod_file`) and `src/process_manager.rs` via `syn::parse_file` to append only newly introduced `pub mod` items or missing `has_ended` / `OnEvent<E>` / `OnTimer<T>` impl blocks.
- **Declarative Transport Precedence** (`src/main.rs`): When `kind: presentation-binding` documents exist in `.infra.esdm.yaml`, `ActiveTransports` is derived solely from `doc.transport` (`http` | `http-rest`, `cli`, `graphql`, `tauri-ipc`, `mcp`), overriding Cargo transport feature defaults (`typescript` generation remains gated by `#[cfg(feature = "typescript")]`).

## Side Effects
- **Filesystem Reads & Writes** (`src/main.rs`, `src/generator/*`): Reads input `.esdm.yaml` documents (and optional `branding/logo-*.jpg` assets when scaffolding `tauri-app`), creates/updates `<out-dir>/<domain-name>/` directory trees, removes stale `presentation/tauri-app/vite.config.js` if present, and writes optional `--emit-graph` JSON files.
- **Subprocess Formatting** (`src/main.rs`): Spawns `cargo fmt` with `current_dir` set to `<out-dir>/<domain-name>/` after workspace emission (logging a warning if `cargo fmt` cannot be spawned).
- **Local HTTP Visualizer Socket** (`src/main.rs:run_visualizer_server`): When invoked with `chromatofoor-gen visualize`, binds `std::net::TcpListener` on `127.0.0.1:0` and serves embedded HTML + `/graph.json` over HTTP/1.1.

## Generator Support Matrix

### Core ESDM Schema Kinds (`schemas/core/v1.yaml`)

| Schema `kind`                  | Supported? | Code Generation Status / Notes                                                                                                                                                              |
| :----------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `domain`                       |     ✅     | Generates the Cargo workspace, shared `domain` crate (`#[chromatofoor::domain]`), and `bootstrap` crate (`Domain::build_with_store` returning `DomainRuntime`).                             |
| `subdomain`                    |     ✅     | Emits `#[chromatofoor::subdomain]` declarations in `bootstrap` and routes bounded contexts into `bounded-contexts/<subdomain>/<bc>/` (unless labeled `tier: "shared-kernel"`).              |
| `bounded-context`              |     ✅     | Generates an isolated library crate (`shared-kernels/<bc>/` or `bounded-contexts/[<subdomain>/]<bc>/`) using `#[chromatofoor::bounded_context(scope = <Domain>)]`, with `generated/`, `domain/`, and conditional `application/read_models/` & `infrastructure/`. |
| `aggregate`                    |     ✅     | Generates `#[chromatofoor::aggregate]` state structs/enums (`architecturekit::State`), `identified_by` metadata, and typed domain errors.                                                   |
| `command`                      |     ✅     | Generates `#[chromatofoor::command]` enums and payload structs with `validator` constraints, `creates` lifecycle flags, and manual `Command::decide` stubs.                                 |
| `event`                        |     ✅     | Generates `#[chromatofoor::event]` enums and payload structs (`architecturekit::Event`/`Events`) and manual `Event::evolve` stubs.                                                          |
| `dynamic-consistency-boundary` |     ✅     | Generates `#[chromatofoor::dynamic_consistency_boundary]` units that fold `consults` events via `Event<Dcb>` directly from `architecturekit::Store`.                                        |
| `read-model`                   |     ✅     | Generates `#[chromatofoor::read_model]` projections backed by `DocumentView<Record>` (`architecturekit::InMemoryView`) and manual `Project<E>` stubs.                                       |
| `query`                        |     ✅     | Generates `#[chromatofoor::query]` structs and `Query<ReadModel>` implementations registered on `DomainBuilder`.                                                                            |
| `entity`                       |     ✅     | Generates `#[chromatofoor::entity]` structs with `identified_by` (`schema` / `static`) and `validator` constraints.                                                                         |
| `value-object`                 |     ✅     | Generates `#[chromatofoor::value_object]` structs with schema validation constraints.                                                                                                       |
| `domain-service`               |     ✅     | Generates `#[chromatofoor::domain_service]` async traits, request/response DTOs, error enums, and manual implementation stubs.                                                              |
| `actor`                        |     ✅     | Generates `#[chromatofoor::actor]` unit structs and wires fail-closed `Authorized` actor lists across commands and queries.                                                                 |
| `policy`                       |     ✅     | Generates an `integration/policies` crate using `#[chromatofoor::policy]` and manual `React<E>` implementations returning typed `<Policy>Commands`.                                         |
| `event-handler`                |     ✅     | Generates an `integration/event-handlers` crate using `#[chromatofoor::event_handler]` and manual `Handle<E>` implementations for outbound side effects.                                    |
| `process-manager`              |     ✅     | Generates `integration/process-managers/<pm>` crates using `#[chromatofoor::process_manager]` (`OnEvent`, `OnTimer`) with `ProcessTimer` scheduling and store-backed state persistence.     |
| `external-system`              |     ✅     | Emits `#[chromatofoor::external_system]` declarations in `domain` and scaffolds async port traits in `src/infrastructure/`.                                                                 |
| `context-mapping`              |     ✅     | Emits `#[chromatofoor::context_mapping]` relationships in `bootstrap` and resolves directional asymmetric `Cargo.toml` dependencies + external-system infrastructure stubs.                 |

### Extension Schemas & Capabilities

| Extension / Feature                                         | Supported? | Code Generation Status / Notes                                                                                                                           |
| :---------------------------------------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aggregate-lifecycle` (`schemas/lifecycle/v1.yaml`)         |     ✅     | Validates non-empty `created` commands per stateful aggregate and emits `creates` on creation `#[chromatofoor::command]` structs for stream preconditions. |
| `feature` BDD Scenarios (`schemas/given-when-then/v1.yaml`) |     ✅     | Generates `#[tokio::test]` suites using `chromatofoor::testing::{given, given_process_manager, given_read_model}`.                                       |
| `presentation-binding` (`schemas/infrastructure/v1.yaml`)   |     ✅     | Selects active transport crates (`api-server`, `interactive-tui`, `graphql-server`, `mcp-server`, `tauri-app`) from `doc.transport`.                     |
| JSON Schema Types & Formats                                 |     ✅     | Maps `uuid` → `uuid::Uuid`, `identifier` → `chromatofoor::Identifier`, `date` / `date-time` → `chrono`, nested objects, arrays, and string `enum`s.      |
| JSON Schema Validation                                      |     ✅     | Emits `validator::Validate` attributes (`length`, `range`, and `nested` validation) enforced before `Command::decide`.                                   |
| Dual-Transport TypeScript SDK (`typescript-client/`)        |     ✅     | Generates typed `types.ts`, `ChromatofoorBridge` (`WebBridge` + `TauriBridge` + `MockBridge`), `DomainClient`, and Svelte 5 Runes state factories.       |

## Verification
```bash
cargo test -p chromatofoor-gen --all-features
cargo clippy -p chromatofoor-gen --all-features -- -D warnings
```
