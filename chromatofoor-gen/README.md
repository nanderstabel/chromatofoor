# Chromatofoor Code Generator (`chromatofoor-gen`)

`chromatofoor-gen` is the deterministic compiler for the Chromatofoor Event Sourcing / CQRS / DDD platform. It parses declarative `ESDM` YAML specifications (`model.esdm.yaml` alongside `.lifecycle.esdm.yaml`, `.feature.esdm.yaml`, and `.infra.esdm.yaml` extensions) and generates strongly-typed Rust workspaces, TypeScript SDKs, and Given-When-Then test suites targeting the `chromatofoor` v2 (`architecturekit`) runtime.

## Generator Support Matrix

### Core ESDM Schema Kinds (`schemas/core/v1.yaml`)

| Schema `kind`                  | Supported? | Code Generation Status / Notes                                                                                                                                                              |
| :----------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `domain`                       |     ✅     | Generates the Cargo workspace, shared `domain` crate (`#[chromatofoor::domain]`), and `bootstrap` crate (`Domain::build_with_store` returning `DomainRuntime`).                             |
| `subdomain`                    |     ✅     | Emits `#[chromatofoor::subdomain]` declarations in `bootstrap` and routes bounded contexts into `bounded-contexts/<subdomain>/<bc>/` directories when declared.                             |
| `bounded-context`              |     ✅     | Generates an isolated library crate (`#[chromatofoor::bounded_context(scope = ::domain::<Domain>)]`) with `generated/`, `domain/`, `application/read_models/`, and `infrastructure/`.       |
| `aggregate`                    |     ✅     | Generates `#[chromatofoor::aggregate]` state structs/enums (`architecturekit::State`), `identified_by` metadata, and typed domain errors.                                                   |
| `command`                      |     ✅     | Generates `#[chromatofoor::command]` enums and payload structs with `validator` constraints, lifecycle annotations, and manual `Command::decide` stubs.                                     |
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
| `context-mapping`              |     ✅     | Emits `#[chromatofoor::context_mapping]` relationships in `bootstrap`, resolves cross-context `Cargo.toml` dependencies, and scaffolds external-system infrastructure stubs.                |

### Extension Schemas & Capabilities

| Extension / Feature                                      | Supported? | Code Generation Status / Notes                                                                                                                           |
| :------------------------------------------------------- | :--------: | :------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `aggregate-lifecycle` (`schemas/lifecycle/v1.yaml`)      |     ✅     | Validates non-empty `created` commands per stateful aggregate and emits `#[lifecycle(create)]` pristine-stream vs. populated-subject preconditions.       |
| `feature` BDD Scenarios (`schemas/given-when-then/v1.yaml`) |  ✅     | Generates `#[tokio::test]` suites using `chromatofoor::testing::{given, given_process_manager, given_read_model}`.                                       |
| `presentation-binding` (`schemas/infrastructure/v1.yaml`) |    ✅     | Declaratively scaffolds `api-server` (`chromatofoor-http`), `interactive-tui`, `graphql-server`, `mcp-server`, `tauri-app`, and custom HTTP spec routes. |
| JSON Schema Types & Formats                              |     ✅     | Maps `uuid` → `uuid::Uuid`, `identifier` → `chromatofoor::Identifier`, `date` / `date-time` → `chrono`, nested objects, arrays, and string `enum`s.      |
| JSON Schema Validation                                   |     ✅     | Emits `validator::Validate` attributes (`length`, `range`, regex patterns, nested validation) enforced before `Command::decide`.                         |
| Dual-Transport TypeScript SDK (`typescript-client/`)     |     ✅     | Generates typed `types.ts`, `ChromatofoorBridge` (`WebBridge` + `TauriBridge`), and `DomainClient`.                                                      |
