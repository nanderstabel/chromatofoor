---
title: Project-Wide Agent Rules
type: agent_rules
last_updated: 2026-11-14
context: Global operational constraints for AI agents operating in this repository.
---
# Project-Wide Agent Rules

## Elephant-Goldfish Model (EGM) Specifications & Contracts

This repository uses the **Elephant-Goldfish Model (EGM)** for architectural specifications, reviews, and codebase contracts:

- **Living Design Specifications**: Architectural designs, trade-offs, and implementation plans live in `docs/designs/` (indexed in `docs/designs/SITEMAP.md`), not in append-only ADR logs.
- **Component Contracts ("Peanuts & Hay")**: Root `README.md` and crate-level `README.md` files document code-grounded `Interface`, `Invariants`, `Side Effects`, and `Verification` contracts. Keep them synchronized with source changes and verify alignment with `/egm:egm-drift-report`.
- **Immutable Upstream Core Schema**: Never modify `schemas/core/v1.yaml`. Modeling extensions belong in dedicated extension schemas under `schemas/<extension>/v1.yaml`.

## Utilizing Codebase Graphs (graphify-rs)

To navigate the codebase efficiently without reading unnecessary files, use the structural graph built by `graphify-rs`:

- Rebuild after adding/removing source files or making structural changes:
  ```bash
  ./scripts/build_graph.sh
  ```
- Output lives in `~/.graphify-rs/Chromatofoor-<hash>/` (`GRAPH_REPORT.md`, `graph.json`, `graph.html`).

## Deterministic Code Generation

When modifying `chromatofoor-gen`, all generated outputs MUST be 100% deterministic:

- **Sorting**: Sort all lists derived from `HashMap` or `HashSet` alphabetically before emitting `Cargo.toml` members, `pub mod` declarations, enum variants, or struct fields.
- **Non-Destructive Boundaries**: Overwrite only `src/generated/`, `bootstrap/`, `domain/`, `tests/`, and generated presentation entry points; never clobber existing developer-authored logic in `src/domain/`, `src/application/read_models/`, `src/infrastructure/`, or manual `process_manager.rs` files.

## Invoking the Code Generator for Testing

When verifying `chromatofoor-gen` changes against a sample domain model from the repository root, use:

```bash
cargo run -p chromatofoor-gen -- \
  --input testbed/04-virtual-apartment/model.esdm.yaml \
  --features testbed/04-virtual-apartment/model.feature.esdm.yaml \
  --lifecycle testbed/04-virtual-apartment/model.lifecycle.esdm.yaml \
  --out-dir testbed/04-virtual-apartment/workspace \
  --chromatofoor-path ../../../../chromatofoor
```

## Running the Verification Suite

Before considering any feature, refactoring, generator change, or ESDM model edit complete, run the full verification suite from the repository root and ensure it exits with status `0`:

```bash
./scripts/verify_all.sh
```
