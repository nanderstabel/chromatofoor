# Chromatofoor Test-Driven Development (TDD) Workflow

Chromatofoor enforces a schema-driven Test-Driven Development (TDD) workflow. Rather than hand-writing test boilerplate after implementing business logic, `chromatofoor-gen` compiles declarative Given-When-Then specifications (`model.feature.esdm.yaml`) directly into executable Rust test suites powered by `chromatofoor::testing`.

## The Red-Green-Refactor Cycle

### 1. Model the Domain & Lifecycles (`model.esdm.yaml` & `model.lifecycle.esdm.yaml`)
Define the structural building blocks of your domain:
- **Aggregates & Dynamic Consistency Boundaries**: Consistency units and their `identifiedBy` strategies (`generated`, `state`, `command-payload`, `static`).
- **Lifecycle (`model.lifecycle.esdm.yaml`)**: Declare which commands initialize new aggregate instances (`created: [...]`) and optionally terminate them (`deleted: [...]`).
- **Commands & Events**: Explicit payloads (using `format: uuid` or `format: identifier` for IDs), validation rules, and permitted `actors`.
- **Read Models, Queries, Policies, & Process Managers**: Projections, read queries, reactive policies, and cross-context sagas.

### 2. Specify Behavioral Scenarios (`model.feature.esdm.yaml`)
Capture business invariants in `model.feature.esdm.yaml` using `kind: feature` across `aggregate`, `dynamicConsistencyBoundary`, `processManager`, or `readModel` scopes:
- **`given`**: Historical domain events folded into initial state.
- **`when`**: The command (with `actor`), incoming event/timer, or query under test.
- **`then`**: Expected emitted events/commands, timer schedules/cancellations, projected read-model JSON, or expected domain/authorization rejection reason.

### 3. Generate the Workspace
Run `chromatofoor-gen` to scaffold or update the target Cargo workspace:
```bash
cargo run -p chromatofoor-gen -- \
  --input model.esdm.yaml \
  --features model.feature.esdm.yaml \
  --lifecycle model.lifecycle.esdm.yaml \
  --out-dir workspace \
  --chromatofoor-path ../chromatofoor
```
The generator overwrites `src/generated/` and `tests/*_tests.rs` while leaving existing manual files in `src/domain/` and `src/application/read_models/` untouched.

### 4. Red Phase (Failing Specifications)
Run `cargo test` inside the generated workspace or bounded context:
```bash
cd workspace/<domain-name>
cargo test
```
Newly scaffolded stubs return default/empty results, causing the generated `chromatofoor::testing` assertions to fail first.

### 5. Green Phase (Pure Functional Implementation)
Implement the domain logic in the manual modules until every generated scenario passes:
- **Command Decisions (`src/domain/<unit>/command/<command>.rs`)**: Implement `Command<Unit>::decide(self, state: &Unit)` to validate invariants and return `Ok(vec![...])` or `Err(...)`.
- **State Evolution (`src/domain/<unit>/event/<event>.rs`)**: Implement `Event<Unit>::evolve(&self, state: Unit) -> Unit` as a pure state transition.
- **Read-Model Projections (`src/application/read_models/<rm>.rs`)**: Implement `Project<Event>::project(&self, event: Envelope<Event>)` to update `DocumentView<T>` / `InMemoryView`.
- **Process Managers (`integration/process-managers/<pm>/src/process_manager.rs`)**: Implement workflow transitions and command construction.

### 6. Evolve & Verify Determinism
When requirements change:
1. Update `model.esdm.yaml`, `model.lifecycle.esdm.yaml`, and `model.feature.esdm.yaml`.
2. Re-run `chromatofoor-gen` to update generated types and `tests/`.
3. Update manual domain handlers until `cargo test` and `./scripts/verify_all.sh` pass cleanly.
