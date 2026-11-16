# ESDM (`esdm`)

## Overview
`esdm` defines the strongly-typed Rust Abstract Syntax Tree (AST), cross-document semantic linter, and optional top-level schema translators for executable **Event-Sourced Domain Model (ESDM)** specifications (`schemas/core/v1.yaml` and sibling execution extensions `lifecycle/v1`, `given-when-then/v1`, and `infrastructure/v1`; narrative `domain-storytelling/v1` documents are not modeled by `EsdmDoc`). It serves as the shared specification parser and semantic validator consumed by `chromatofoor-gen` and schema verification harnesses.

## Interface
- **Cargo Feature Flags** (`Cargo.toml`):
  - `openapi` — Enables `dep:utoipa` and `esdm::openapi::ToOpenApi`.
  - `jsonschema` — Enables `dep:schemars` and `esdm::jsonschema::ToJsonSchema`.
  - `typescript` — Enables `esdm::typescript::ToTypeScript`.
- **Core Document & Schema AST** (`src/lib.rs`):
  - `EsdmDoc` — Root camelCase-serialized ESDM document modeling executable `kind`s (`domain`, `subdomain`, `bounded-context`, `aggregate`, `dynamic-consistency-boundary`, `command`, `event`, `read-model`, `query`, `entity`, `value-object`, `domain-service`, `actor`, `policy`, `event-handler`, `process-manager`, `external-system`, `context-mapping`, `aggregate-lifecycle`, `feature`, `presentation-binding`; omits informational fields such as `apiVersion`, `sideEffects`, `paradigm`, `responsibilities`, `backedBy`, and `capabilities`).
  - `Metadata`, `Scope` — Document labels/annotations (`IndexMap<String, String>`) and hierarchical scope descriptors (`domain`, `bounded_context`, `aggregate`, `dynamic_consistency_boundary`, `read_model`, `process_manager`, `command`, `query`).
  - `DataSchema`, `PropertySchema` — Recursive object and property JSON Schema representations supporting `$ref`, `type`, `format`, `enum`, `items`, nested `properties`/`required`, and numeric/length/pattern validation constraints.
  - `IdentifiedBy`, `IdentifiedByEntry`, `CorrelatedBy`, `Idempotency` — Identity source (`Single` or `Multiple` entries with `as_slice`), correlation, and idempotency descriptors.
  - `DomainFunction`, `Invariant`, `Projection`, `Consult`, `MappingEnd`, `EventRef`, `CommandRef` — Domain behavior, read-model projection rules, DCB consultation filters, and cross-context references.
  - `ReactionWhen`, `Reaction`, `NamedCondition`, `Timer`, `TimerAfter` — Process-manager lifecycle triggers, reactions, termination conditions, and relative/scheduled timer specifications.
  - `RestBinding`, `CliBinding`, `GraphqlBinding`, `TauriIpcBinding` — Declarative transport bindings attached to commands, queries, and presentation specs.
  - `Scenario`, `GivenEvent`, `WhenAction`, `ExpectedCommand`, `ThenExpectation`, `RejectionExpectation` — Declarative Given-When-Then behavioral test scenario AST (`given-when-then/v1`).
- **Cross-Document Semantic Linter** (`src/lint.rs`):
  - `lint::validate_model(docs: &[EsdmDoc]) -> Vec<LintViolation>` — Validates cross-document `lifecycle/v1` completeness and identifier field contracts across aggregates, DCBs, entities, and commands.
  - `lint::LintViolation` — Structured diagnostic (`rule_id`, `kind`, `name`, `bounded_context`, `message`) implementing `Display`, `PartialEq`, and `Eq`.
- **Optional Shallow Schema Translators** (inspect top-level `DataSchema::properties` and `required`):
  - `jsonschema::ToJsonSchema` (`src/jsonschema.rs`, feature `jsonschema`) — `to_json_schema(&self) -> schemars::schema::Schema` implemented for `DataSchema`.
  - `openapi::ToOpenApi` (`src/openapi.rs`, feature `openapi`) — `to_openapi_schema(&self) -> utoipa::openapi::schema::Schema` implemented for `DataSchema`.
  - `typescript::ToTypeScript` (`src/typescript.rs`, feature `typescript`) — `to_typescript(&self, name: &str) -> String` implemented for `DataSchema`.

## Invariants
- **Strict Rust Safety & Lint Baseline** (`Cargo.toml`): Inherits workspace lints enforcing `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)`.
- **Deterministic Property Ordering** (`src/lib.rs`): `DataSchema::properties`, `PropertySchema::properties`, `Metadata`, and `EsdmDoc::config` use `IndexMap` so modeled properties preserve exact YAML/JSON declaration order.
- **Aggregate Lifecycle Completeness & Disjointness** (`src/lint.rs:validate_model`):
  - Every `kind: aggregate-lifecycle` document must declare both `scope.bounded_context` and `scope.aggregate` (`esdm/lifecycle/missing-scope`), target an existing aggregate (`esdm/lifecycle/unknown-aggregate`) without duplicates (`esdm/lifecycle/duplicate-aggregate-lifecycle`), and specify a non-empty `created` command list (`esdm/lifecycle/empty-created`).
  - Referenced `created`, `mutated`, and `deleted` commands must exist on the target aggregate (`esdm/lifecycle/unknown-*-command`) and be pairwise disjoint (`esdm/lifecycle/conflicting-command`).
  - Any command-bearing stateful aggregate (`state`, `generated`, empty, or omitted `identified_by`) must have a matching `aggregate-lifecycle` with at least one `created` command (`esdm/lifecycle/missing-aggregate-lifecycle`).
- **Identifier Schema & Cross-Document Format Alignment** (`src/lint.rs:validate_model`, `src/lint.rs:check_id_field`):
  - Required identifier fields must exist in `properties`, appear in `required`, declare `type: "string"`, and use `format: "uuid"` or `format: "identifier"` across `aggregate.state` (`esdm/identity/aggregate-state-id-field`), `command-payload` DCB commands (`esdm/identity/dcb-command-id-field`), and `schema`-identified entities (`esdm/identity/entity-schema-id-field`).
  - Commands on `state`-identified aggregates must carry the state ID field with the exact same `format` declared on `aggregate.state` (`esdm/identity/command-missing-state-id-field`); non-creation commands on `generated`-identity aggregates must declare field `"id"` with `format: "uuid"` (when `generator` is `"uuid"` or omitted) or `"identifier"` (`esdm/identity/command-missing-generated-id-field`).

## Side Effects
- None. `esdm` performs pure in-memory serde AST deserialization/serialization, semantic validation, and schema translation with zero filesystem, network, or process I/O.

## Verification
```bash
cargo test -p esdm --all-features
cargo clippy -p esdm --all-features -- -D warnings
```
