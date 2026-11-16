<p align="center">
  <img src="./branding/logo-horizontal.jpg" width="640" alt="Chromatofoor">
</p>

<p align="center">
  <a href="https://github.com/nanderstabel/chromatofoor/actions/workflows/ci.yml"><img src="https://github.com/nanderstabel/chromatofoor/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/Rust-1.95%20%28edition%202024%29-orange.svg" alt="Rust 1.95 (edition 2024)">
  <img src="https://img.shields.io/badge/unsafe__code-deny-brightgreen.svg" alt="unsafe_code deny">
  <img src="https://img.shields.io/badge/license-MIT%20%2F%20AGPL--3.0-blue.svg" alt="License: MIT / AGPL-3.0">
</p>

# Chromatofoor Blueprint

## System Overview
Chromatofoor is a Rust Domain-Driven Design (DDD), CQRS, and Event Sourcing platform built on `architecturekit`. It compiles declarative ESDM (`Event-Sourced Domain Model`) YAML specifications—or hand-authored Rust `#[chromatofoor::<kind>]` attribute macros—into `#![deny(unsafe_code)]` Cargo workspaces, composable presentation servers (`HTTP/SSE`, `GraphQL`, `MCP`, `TUI`, `Tauri v2`), Given-When-Then test suites (`chromatofoor::testing`), and dual-transport TypeScript SDKs.

Living design specifications and architectural decisions live in [`docs/designs/SITEMAP.md`](./docs/designs/SITEMAP.md). Schema-driven TDD workflow instructions live in [`docs/tdd-workflow.md`](./docs/tdd-workflow.md).

## Quick Example: Canonical Macro & ESDM Parity
From [`testbed/08-digital-identity-ecosystem`](./testbed/08-digital-identity-ecosystem/model.esdm.yaml), a single declarative command specification compiles deterministically into canonical `#[chromatofoor::<kind>]` Rust with compile-time scope, lifecycle, validation, and fail-closed actor checks:

```yaml
apiVersion: schema.esdm.io/core/v1
kind: command
name: generate-signing-key
actors: [organization-admin]
scope:
  domain: digital-identity-ecosystem
  boundedContext: organization-identity
  aggregate: organization-keyring
data:
  type: object
  properties:
    algorithm: { type: string }
    purpose: { type: string }
  required: [algorithm, purpose]
```

```rust
#[chromatofoor::aggregate(
    scope = crate::generated::domain::OrganizationIdentity,
    identified_by = [generated(uuid)],
)]
#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub enum OrganizationKeyring {
    #[default]
    None,
    Created(OrganizationKeyringInner),
}

#[chromatofoor::command(
    scope = crate::generated::domain::organization_keyring::aggregate::OrganizationKeyring,
    creates,
    validate,
    actors = [crate::generated::domain::actors::OrganizationAdmin],
)]
pub struct GenerateSigningKey {
    pub algorithm: String,
    pub purpose: String,
}
```

## Responsible AI-First Rust Engineering (Elephant-Goldfish Model)
Instead of unconstrained "vibe coding", Chromatofoor pairs the **Elephant-Goldfish Model (EGM)** with strict compiler, lint, and runtime guardrails so AI agents can design, implement, test, and audit safely:

- **Living Architectural Memory ("Elephant")**: Design specs and trade-offs live in [`docs/designs/SITEMAP.md`](./docs/designs/SITEMAP.md), operational constraints in [`.agents/AGENTS.md`](./.agents/AGENTS.md), and code-grounded crate contracts ("Peanuts & Hay") in each core crate's `README.md`.
- **Zero-Context Verification ("Goldfish")**: Subagents audit crate contracts and designs in fresh contexts against source files (`egm-codebase-details`, `egm-structural`, `egm-drift-report`), preventing doc-to-code drift and context-window rot.
- **Compiler & Static Guardrails**: Workspace lints enforce `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)` across crates; `chromatofoor-derive` verifies scope hierarchies, `Identified` fields, and `Aggregate` lifecycle preconditions at compile time.
- **Deterministic Codegen & Non-Destructive Boundaries**: `chromatofoor-gen` validates models via `esdm::lint::validate_model` before emission, parses generated token streams via `syn::parse_file`, sorts outputs deterministically, and preserves developer-authored logic in `src/domain/`, `src/application/read_models/`, and `src/infrastructure/`.
- **Fail-Closed Runtime & Schema-Driven BDD**: `DomainRuntime` enforces two-phase `Authorized` actor checks (rejecting unauthorized callers before JSON deserialization), while `chromatofoor::testing` executes Given-When-Then suites compiled from `.feature.esdm.yaml`.

## Spotlights
- **Native Model Context Protocol Adapter ([`presentation/mcp/`](./presentation/mcp/README.md))**: `chromatofoor-mcp` projects any `DomainRuntime` into an MCP JSON-RPC 2.0 server over standalone `stdio` (`Mcp::stdio()`) or Axum Streamable HTTP + live `notifications/domain_event` SSE (`Mcp::http()`). Commands (`invoke_*`) and queries (`query_*`) automatically carry `_actor`-enriched JSON Schemas so AI agents operate under the exact same fail-closed authorization rules as human callers.
- **Digital Identity Ecosystem Reference Domain ([`testbed/08-digital-identity-ecosystem`](./testbed/08-digital-identity-ecosystem/model.esdm.yaml))**: End-to-end multi-context ESDM specification and verified Cargo workspace modeling **OID4VCI** (credential issuance), **OID4VP** (verifiable presentation & verification), **OpenID Federation**, and **Token Status List** with declarative `model.infra.esdm.yaml` HTTP bindings, cross-context policies, and Given-When-Then BDD scenarios.

## Subdirectories & System Boundaries
- **Core Compiler & Runtime Crates** *(primary EGM scope)*:
  - [`esdm/`](./esdm/README.md) — ESDM YAML AST, JSON Schema validation (`schemas/`), and cross-document semantic linter (`esdm::lint`).
  - [`chromatofoor/`](./chromatofoor/README.md) — Core pure-functional DDD/CQRS/ES runtime (`Aggregate`, `DynamicConsistencyBoundary`, `Command`, `Event`, `ReadModel`, `DocumentView`, `Query`, `Policy`, `EventHandler`, `ProcessManager`, `DomainBuilder`, `DomainRuntime`, `testing`).
  - [`chromatofoor-derive/`](./chromatofoor-derive/README.md) — Procedural attribute macros (`#[chromatofoor::<kind>]`) enforcing compile-time scope hierarchy, `Identified` field validation, lifecycle preconditions, and `Authorized` actor checks.
  - [`chromatofoor-gen/`](./chromatofoor-gen/README.md) — Deterministic workspace, domain, integration, BDD test, presentation, and TypeScript SDK code generator.
- **Composable Presentation & UI Adapters**:
  - `presentation/http/` (`chromatofoor-http`) — HTTP (`GET`, `POST`, `SSE`) + OpenAPI server (`HttpServer`) and `HttpProtocol` sub-router host.
  - `presentation/graphql/` (`chromatofoor-graphql`) — Dynamic GraphQL schema, mutations, queries, and subscriptions (`HttpProtocol`).
  - [`presentation/mcp/`](./presentation/mcp/README.md) (`chromatofoor-mcp`) — Model Context Protocol server over `stdio` (`Presentation`) and Streamable HTTP (`HttpProtocol`).
  - `presentation/tui/` (`chromatofoor-tui`) — Ratatui interactive terminal UI (`Presentation`).
  - `presentation/tauri/` (`chromatofoor-tauri`) — Tauri v2 desktop IPC commands and event streaming bridge.
  - `packages/ui/` — `@chromatofoor/ui` Svelte 5 atomic component library and W3C design tokens.
- **Verification Suites**:
  - [`testbed/`](./testbed/README.md) — Reference ESDM domains (`01`–`08`, including [`08-digital-identity-ecosystem`](./testbed/08-digital-identity-ecosystem/model.esdm.yaml) for OID4VCI, OID4VP, OpenID Federation, and Token Status List) verifying schema linting, runtime semantics, and generator determinism.

## Global Invariants
- **Pure Functional Domain Core**: `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)` across workspace crates (`chromatofoor-gen/src/main.rs` carries targeted crate-local `unwrap_used`/`expect_used` allowances, and `chromatofoor/src/testing.rs` permits `panic` in assertion helpers).
- **Canonical Macro Parity**: `chromatofoor-gen` emits the exact same `#[chromatofoor::<kind>]` macros and traits used by hand-written Rust domains, sharing a generated `<workspace>/domain` crate.
- **Fail-Closed Authorization**: Commands and queries without declared `actors` reject all actor-checked dispatch (`DispatchError::Unauthorized`).
- **Deterministic Non-Destructive Code Generation**: All generated collections and workspace members are sorted deterministically (while `IndexMap`-backed read-model, query, and process-manager fields preserve YAML declaration order); `src/generated/` and `tests/` are overwritten cleanly while manual domain files (`src/domain/`, `src/application/read_models/`, `src/infrastructure/`) are preserved.
- **Immutable Core Schema**: `schemas/core/v1.yaml` is the upstream ESDM core specification; execution extensions (`lifecycle/v1`, `given-when-then/v1`, `infrastructure/v1`, `domain-storytelling/v1`) live in sibling extension schemas.

## Global Side Effects
- Event persistence, optimistic concurrency preconditions, and projection/consumer subscriptions run through `architecturekit::Store` (`EventStore` implementations such as EventSourcingDB or `InMemoryEventStore`).
- Consumer checkpoints persist to `/checkpoints/<kind>/<domain>/<consumer>`, Process Manager snapshots and outbox `pending_commands` persist to `/process-managers/<domain>/<pm>/<id>`, and fired timer events persist to `/<domain>/<pm>/timers/<id>`.

## System Verification
- Run full workspace tests, strict clippy, ESDM model linting, testbed tests, regeneration, and git-diff determinism checks:
  ```bash
  ./scripts/verify_all.sh
  ```

## License
- **Runtime, Macros, Linter, Transports, UI & Testbeds** (`chromatofoor`, `chromatofoor-derive`, `esdm`, `presentation/*`, `packages/ui`, `testbed/*`): Licensed under the [MIT License](./LICENSE).
- **Code Generator** (`chromatofoor-gen`): Licensed under [GNU AGPL-3.0](./chromatofoor-gen/LICENSE) with the [Generated Code Output Exception](./chromatofoor-gen/OUTPUT_EXCEPTION.md) (generated workspaces and application code are unencumbered and owned by you).
- **Upstream ESDM Schemas** (`schemas/core/v1.yaml`, `schemas/given-when-then/v1.yaml`, `schemas/domain-storytelling/v1.yaml`): Copyright (c) 2026 the native web GmbH, licensed under the MIT License.
