---
title: Project-Wide Agent Rules
type: agent_rules
last_updated: 2026-11-16
context: Global operational constraints for AI agents operating in this repository.
---
# Project-Wide Agent Rules

## Elephant-Goldfish Model (EGM) Specifications & Contracts

This repository uses the **Elephant-Goldfish Model (EGM)** for architectural specifications, reviews, and codebase contracts:

- **Living Design Specifications**: Architectural designs, trade-offs, and implementation plans live in `docs/designs/` (indexed in `docs/designs/SITEMAP.md`), not in append-only ADR logs.
- **Component Contracts ("Peanuts & Hay")**: Root `README.md` and crate-level `README.md` files document code-grounded `Interface`, `Invariants`, `Side Effects`, and `Verification` contracts. Keep them synchronized with source changes and verify alignment with `/egm:egm-drift-report`.
- **Immutable Upstream Core Schema**: Never modify `schemas/core/v1.yaml`. Modeling extensions belong in dedicated extension schemas under `schemas/<extension>/v1.yaml`.

## Deterministic Code Generation

When modifying `chromatofoor-gen`, all generated outputs MUST be 100% deterministic:

- **Sorting**: Sort all lists derived from `HashMap` or `HashSet` alphabetically before emitting `Cargo.toml` members, `pub mod` declarations, enum variants, or struct fields.
- **Non-Destructive Boundaries**: Overwrite only `src/generated/`, `bootstrap/`, `domain/`, `<crate>/tests/generated/`, and generated presentation entry points; never clobber existing developer-authored logic in `src/domain/`, `src/application/read_models/`, `src/infrastructure/`, manual `process_manager.rs` files, or manual `<crate>/tests/*.rs` files.

## Invoking the Code Generator for Testing

When verifying `chromatofoor-gen` changes against a sample domain model from the repository root (`model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `model.infra.esdm.yaml` are auto-discovered alongside `model.esdm.yaml`):

```bash
cargo run -p chromatofoor-gen --features tui -- \
  --input testbed/04-flight-booking/model.esdm.yaml \
  --out-dir testbed/04-flight-booking/workspace \
  --chromatofoor-path ../../../../chromatofoor
```

## Running the Verification Suite

Before considering any feature, refactoring, generator change, or ESDM model edit complete, run the full verification suite from the repository root and ensure it exits with status `0`:

```bash
./scripts/verify_all.sh
```
