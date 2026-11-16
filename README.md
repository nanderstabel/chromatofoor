# Chromatofoor Blueprint

## System Overview
Chromatofoor is a Rust Domain-Driven Design (DDD), CQRS, and Event Sourcing platform built on `architecturekit`. It compiles declarative ESDM (`Event-Sourced Domain Model`) YAML specifications—or hand-authored Rust `#[chromatofoor::<kind>]` attribute macros—into `#![deny(unsafe_code)]` Cargo workspaces, composable presentation servers, Given-When-Then test suites, and dual-transport TypeScript SDKs.

Living design specifications and architectural decisions live in [`docs/designs/SITEMAP.md`](./docs/designs/SITEMAP.md). Schema-driven TDD workflow instructions live in [`docs/tdd-workflow.md`](./docs/tdd-workflow.md).

## Subdirectories & System Boundaries
- **Core Compiler & Runtime Crates** *(primary EGM scope)*:
  - `esdm/` — ESDM YAML AST, JSON Schema validation (`schemas/`), and cross-document semantic linter (`esdm::lint`).
  - `chromatofoor/` — Core pure-functional DDD/CQRS/ES runtime (`Aggregate`, `DynamicConsistencyBoundary`, `Command`, `Event`, `ReadModel`, `DocumentView`, `Query`, `Policy`, `EventHandler`, `ProcessManager`, `DomainBuilder`, `DomainRuntime`, `testing`).
  - `chromatofoor-derive/` — Procedural attribute macros (`#[chromatofoor::<kind>]`) enforcing compile-time scope hierarchy, `Identified` field validation, lifecycle preconditions, and `Authorized` actor checks.
  - [`chromatofoor-gen/`](./chromatofoor-gen/README.md) — Deterministic workspace, domain, integration, BDD test, presentation, and TypeScript SDK code generator.
- **Composable Presentation & UI Adapters**:
  - `presentation/http/` (`chromatofoor-http`) — HTTP (`GET`, `POST`, `SSE`) + OpenAPI server (`HttpServer`) and `HttpProtocol` sub-router host.
  - `presentation/graphql/` (`chromatofoor-graphql`) — Dynamic GraphQL schema, mutations, queries, and subscriptions (`HttpProtocol`).
  - `presentation/mcp/` (`chromatofoor-mcp`) — Model Context Protocol server over `stdio` (`Presentation`) and Streamable HTTP (`HttpProtocol`).
  - `presentation/tui/` (`chromatofoor-tui`) — Ratatui interactive terminal UI (`Presentation`).
  - `presentation/tauri/` (`chromatofoor-tauri`) — Tauri v2 desktop IPC commands and event streaming bridge.
  - `packages/ui/` — `@chromatofoor/ui` Svelte 5 atomic component library and W3C design tokens.
- **Verification Suites**:
  - [`testbed/`](./testbed/README.md) — Reference ESDM domains (`01`–`08`, including `08-digital-identity-ecosystem` for OID4VCI, OID4VP, OpenID Federation, and Token Status List) verifying schema linting, runtime semantics, and generator determinism.

## Global Invariants
- **Pure Functional Domain Core**: `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)` across workspace crates.
- **Canonical Macro Parity**: `chromatofoor-gen` emits the exact same `#[chromatofoor::<kind>]` macros and traits used by hand-written Rust domains, sharing a generated `<workspace>/domain` crate.
- **Fail-Closed Authorization**: Commands and queries without declared `actors` reject all actor-checked dispatch (`DispatchError::Unauthorized`).
- **Deterministic Non-Destructive Code Generation**: All generated collections are sorted deterministically; `src/generated/` and `tests/` are overwritten cleanly while manual domain files (`src/domain/`, `src/application/read_models/`, `src/infrastructure/`) are preserved.
- **Immutable Core Schema**: `schemas/core/v1.yaml` is the upstream ESDM core specification; execution extensions (`lifecycle/v1`, `given-when-then/v1`, `infrastructure/v1`, `domain-storytelling/v1`) live in sibling extension schemas.

## Global Side Effects
- Event persistence, optimistic concurrency preconditions, and projection/consumer subscriptions run through `architecturekit::Store` (`EventStore` implementations such as EventSourcingDB or `InMemoryEventStore`).
- Process Manager instances and outbox `pending_commands` persist to `/<domain>/process-managers/<pm>/<id>`, and scheduled timers persist to `/<domain>/timers/<timer>/<id>`.

## System Verification
- Run full workspace tests, strict clippy, ESDM model linting, testbed tests, regeneration, and git-diff determinism checks:
  ```bash
  ./scripts/verify_all.sh
  ```

## License
- **Runtime, Macros, Linter, Transports, UI & Testbeds** (`chromatofoor`, `chromatofoor-derive`, `esdm`, `presentation/*`, `packages/ui`, `testbed/*`): Licensed under the [MIT License](./LICENSE).
- **Code Generator** (`chromatofoor-gen`): Licensed under [GNU AGPL-3.0](./chromatofoor-gen/LICENSE) with the [Generated Code Output Exception](./chromatofoor-gen/OUTPUT_EXCEPTION.md) (generated workspaces and application code are unencumbered and owned by you).
- **Upstream ESDM Schemas** (`schemas/core/v1.yaml`, `schemas/given-when-then/v1.yaml`, `schemas/domain-storytelling/v1.yaml`): Copyright (c) 2026 the native web GmbH, licensed under the MIT License.
