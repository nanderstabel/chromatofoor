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
- **Repository Root Blueprint**: [`README.md`](../../../README.md) — High-level crate map, global invariants, and verification commands.
- **Design Sitemap**: [`docs/designs/SITEMAP.md`](../../../docs/designs/SITEMAP.md) — Index of all living EGM design specifications in `docs/designs/`.
- **Schema-Driven TDD Guide**: [`docs/tdd-workflow.md`](../../../docs/tdd-workflow.md) — Red-Green-Refactor workflow from `.esdm.yaml` specs to `chromatofoor::testing`.

## 2. Living Design Specifications (`docs/designs/`)
- **Runtime, Macros & Transports**: [`docs/designs/runtime-and-macros.md`](../../../docs/designs/runtime-and-macros.md) — `chromatofoor`, `chromatofoor-derive`, `esdm`, and composable `Presentation` / `HttpProtocol` architecture.
- **Deterministic Code Generator**: [`docs/designs/code-generator.md`](../../../docs/designs/code-generator.md) — `chromatofoor-gen` crate layout, non-destructive stub boundaries, and [`chromatofoor-gen/README.md`](../../../chromatofoor-gen/README.md) Support Matrix.

## 3. ESDM JSON Schemas (`schemas/`)
- Core DDD/CQRS-ES specification: [`schemas/core/v1.yaml`](../../../schemas/core/v1.yaml)
- Extensions: [`schemas/lifecycle/v1.yaml`](../../../schemas/lifecycle/v1.yaml), [`schemas/given-when-then/v1.yaml`](../../../schemas/given-when-then/v1.yaml), [`schemas/infrastructure/v1.yaml`](../../../schemas/infrastructure/v1.yaml), [`schemas/domain-storytelling/v1.yaml`](../../../schemas/domain-storytelling/v1.yaml)
