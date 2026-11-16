# Design Specifications Sitemap

Index of living technical specifications and architectural records maintained under the Elephant-Goldfish Model (EGM).

## Core Platform Specifications

| Specification | Scope | Summary |
| :--- | :--- | :--- |
| [`runtime-and-macros.md`](./runtime-and-macros.md) | `chromatofoor`, `chromatofoor-derive`, `esdm`, `presentation/*` | Pure-functional `architecturekit` runtime, hierarchical `/<domain>/...` topic paths, compile-time `Identified` & lifecycle preconditions, fail-closed actor authorization, outbox Process Managers, `chromatofoor::testing`, and composable `Presentation` / `HttpProtocol` layers. |
| [`code-generator.md`](./code-generator.md) | `chromatofoor-gen` | Deterministic compilation from ESDM YAML + extension schemas into multi-crate Cargo workspaces, canonical v2 macro parity, shared `<workspace>/domain` crate, non-destructive manual stub boundaries, and dual-transport TypeScript SDK generation. |

## Developer Guides

- [`../tdd-workflow.md`](../tdd-workflow.md) — Schema-driven Red-Green-Refactor workflow using `model.esdm.yaml`, `model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `chromatofoor::testing`.
