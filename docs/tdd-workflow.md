# Chromatofoor Test-Driven Development (TDD) Workflow

Chromatofoor enforces a schema-driven Test-Driven Development (TDD) workflow. Rather than hand-writing test boilerplate after implementing business logic, `chromatofoor-gen` compiles declarative Given-When-Then specifications (`model.feature.esdm.yaml`) into isolated Rust integration test suites (`<crate>/tests/generated/main.rs`) paired with explicit `todo!(...)` implementation stubs.

## The Red-Green-Refactor Cycle

### 1. Model the Domain & Lifecycles (`model.esdm.yaml` & `model.lifecycle.esdm.yaml`)
Define the structural building blocks of your domain:
- **Aggregates & Dynamic Consistency Boundaries**: Consistency units and their `identifiedBy` strategies (`generated`, `state`, `command-payload`, `static`).
- **Lifecycle (`model.lifecycle.esdm.yaml`)**: Declare which commands initialize new aggregate instances (`created: [...]`) and optionally terminate them (`deleted: [...]`).
- **Commands & Events**: Explicit payloads (using `format: uuid` or `format: identifier` for IDs), validation rules, and permitted `actors`.
- **Read Models, Queries, Policies, Event Handlers, & Process Managers**: Projections, read queries, reactive policies, outbound side-effect handlers, and cross-context sagas.

### 2. Specify Behavioral Scenarios (`model.feature.esdm.yaml`)
Capture business invariants in `model.feature.esdm.yaml` using `kind: feature` across `aggregate`, `dynamicConsistencyBoundary`, `processManager`, or `readModel` scopes:
- **`given`**: Historical domain events folded into initial state.
- **`when`**: The command (with optional `actor`), incoming event/timer, or query under test.
- **`then`**: Expected emitted events (`events: []` for idempotent no-ops), emitted saga commands, timer schedules/cancellations, query `result` or `readModel` JSON subsets, or expected domain/authorization `rejection`.

### 3. Generate the Workspace
Run `chromatofoor-gen` to scaffold or update the target Cargo workspace (`model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `model.infra.esdm.yaml` in the same directory as `--input` are auto-discovered automatically):
```bash
cargo run -p chromatofoor-gen -- \
  --input model.esdm.yaml \
  --out-dir workspace \
  --chromatofoor-path ../../chromatofoor
```
The generator overwrites `src/generated/` and `<crate>/tests/generated/` while leaving existing manual files in `src/domain/`, `src/application/read_models/`, `src/infrastructure/`, and hand-written `<crate>/tests/*.rs` untouched.

### 4. Red Phase (Compiling Workspace, Failing `todo!` Specifications & Clippy Burndown)
Every freshly generated workspace compiles out of the box (`cargo test --workspace --no-run`) with `#![deny(unsafe_code)]` and `clippy::todo = "warn"` inherited across all member crates:
```bash
cd workspace/<domain-name>
cargo test --workspace
cargo clippy --workspace --all-targets
```
- `cargo test --workspace` runs `<crate>/tests/generated/main.rs` (`--test generated`) and fails deterministically on every unimplemented scenario with `not yet implemented: implement <Trait>::<fn> for <Type>` (or on validation/authorization/query-result assertions).
- `cargo clippy --workspace --all-targets` lists every remaining `todo!(...)` stub across commands, events, read models, domain services, policies, event handlers, and process managers—even for hooks without a dedicated scenario yet.

### 5. Green Phase (Pure Functional Implementation)
Replace each `todo!(...)` stub in the manual modules until every generated scenario passes and zero `clippy::todo` warnings remain:
- **Command Decisions (`src/domain/<unit>/command/<command>.rs`)**: Implement `Command<Unit>::decide(self, state: &Unit)` to check invariants and return `Ok(vec![...])` or `Err(...)`.
- **State Evolution (`src/domain/<unit>/event/<event>.rs`)**: Implement `Event<Unit>::evolve(&self, state: Unit) -> Unit` as a pure state transition (returning `state` unchanged if a particular event does not affect decision state).
- **Read-Model Projections (`src/application/read_models/<rm>.rs`)**: Implement `Project<Event>::project(&self, event: Envelope<Event>)` to update `DocumentView<T>` / `InMemoryView`.
- **Policies, Event Handlers, & Process Managers (`integration/...`)**: Implement `React::react`, `Handle::handle`, `OnEvent::on_event`, `OnTimer::on_timer`, and `has_ended`.

Verify full Green completion with:
```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings -D clippy::todo
```

### 6. Evolve & Verify Determinism
When requirements change:
1. Update `model.esdm.yaml`, `model.lifecycle.esdm.yaml`, and `model.feature.esdm.yaml`.
2. Re-run `chromatofoor-gen` to synchronize generated types, purge obsolete files under `tests/generated/`, and append any new manual module/reaction stubs.
3. Implement any newly scaffolded `todo!(...)` stubs until `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings -D clippy::todo` pass cleanly.
