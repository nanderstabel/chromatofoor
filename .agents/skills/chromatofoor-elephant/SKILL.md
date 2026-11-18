---
name: chromatofoor-elephant
description: >-
  Loads Chromatofoor project context, architecture blueprints, living EGM design
  specifications, and ESDM schema references. Activate when brainstorming, drafting
  designs, planning features, or navigating Chromatofoor's runtime, macros, or generator.
---

# Chromatofoor Elephant Context Loader

Use these authoritative checked-in artifacts to load context selectively based on the task at hand:

## 1. Blueprints & Developer Workflows
- **Repository Root Overview & Blueprint**: [`README.md`](../../../README.md) — Human-facing quickstart and overview, plus collapsible EGM root blueprint (`Subdirectories & System Boundaries`, `Global Invariants`, `Global Side Effects`, `System Verification`).
- **Component & Subsystem Contracts ("Peanuts & Hay")**:
  - [`esdm/README.md`](../../../esdm/README.md) — ESDM YAML AST, JSON Schema validation, and cross-document semantic linter (`esdm::lint`).
  - [`chromatofoor/README.md`](../../../chromatofoor/README.md) — Core pure-functional DDD/CQRS/ES runtime, outbox Process Managers, and `chromatofoor::testing`.
  - [`chromatofoor-derive/README.md`](../../../chromatofoor-derive/README.md) — `#[chromatofoor::<kind>]` procedural macros enforcing compile-time scope, ID, lifecycle, and actor invariants.
  - [`chromatofoor-gen/README.md`](../../../chromatofoor-gen/README.md) — Deterministic workspace, domain, BDD test, presentation, and TypeScript SDK compiler.
  - [`presentation/README.md`](../../../presentation/README.md) — Composable transport adapters index (`Presentation` & `HttpProtocol`):
    - [`presentation/http/README.md`](../../../presentation/http/README.md) — Axum HTTP (`GET`, `POST`, `SSE`) + OpenAPI server and `HttpProtocol` host (`chromatofoor-http`).
    - [`presentation/graphql/README.md`](../../../presentation/graphql/README.md) — Dynamic GraphQL schema, mutations, queries, and subscriptions (`chromatofoor-graphql`).
    - [`presentation/mcp/README.md`](../../../presentation/mcp/README.md) — Model Context Protocol (`stdio` and Streamable HTTP) adapter (`chromatofoor-mcp`).
    - [`presentation/tui/README.md`](../../../presentation/tui/README.md) — Ratatui interactive terminal UI (`chromatofoor-tui`).
    - [`presentation/tauri/README.md`](../../../presentation/tauri/README.md) — Tauri v2 desktop IPC commands and event-snapshot bridge (`chromatofoor-tauri`).
- **Design Sitemap**: [`docs/designs/SITEMAP.md`](../../../docs/designs/SITEMAP.md) — Index of all living EGM design specifications in `docs/designs/`.
- **Schema-Driven TDD Guide**: [`docs/tdd-workflow.md`](../../../docs/tdd-workflow.md) — Red-Green-Refactor workflow from `.esdm.yaml` specs to `chromatofoor::testing`.

## 2. Living Design Specifications (`docs/designs/`)
- **Runtime, Macros & Transports**: [`docs/designs/runtime-and-macros.md`](../../../docs/designs/runtime-and-macros.md) — `chromatofoor`, `chromatofoor-derive`, `esdm`, and composable `Presentation` / `HttpProtocol` architecture.
- **Deterministic Code Generator**: [`docs/designs/code-generator.md`](../../../docs/designs/code-generator.md) — `chromatofoor-gen` crate layout, non-destructive stub boundaries, and [`chromatofoor-gen/README.md`](../../../chromatofoor-gen/README.md) Support Matrix.

## 3. ESDM JSON Schemas (`schemas/`)
- **Core DDD/CQRS-ES Specification**: [`schemas/core/v1.yaml`](../../../schemas/core/v1.yaml)
- **Extensions**: [`schemas/lifecycle/v1.yaml`](../../../schemas/lifecycle/v1.yaml), [`schemas/given-when-then/v1.yaml`](../../../schemas/given-when-then/v1.yaml), [`schemas/infrastructure/v1.yaml`](../../../schemas/infrastructure/v1.yaml), [`schemas/domain-storytelling/v1.yaml`](../../../schemas/domain-storytelling/v1.yaml)
