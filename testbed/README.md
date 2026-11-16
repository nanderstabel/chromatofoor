# Chromatofoor Testbeds

This directory contains end-to-end reference domains used to verify `chromatofoor`, `chromatofoor-derive`, `chromatofoor-gen`, `esdm`, and all presentation adapters under `./scripts/verify_testbeds.sh`.

Each subdirectory contains a declarative `model.esdm.yaml` alongside its extension specifications (`model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and optional `model.infra.esdm.yaml`) and generated Cargo workspace.

## Testbed Catalog

- **`01-bank-account`**: Foundational banking domain demonstrating aggregates, lifecycle creation/mutation commands, events, actors, and an account summary read model.
- **`02-ecommerce-checkout`**: E-commerce cart and checkout domain exercising JSON Schema validation constraints, explicit rejection scenarios, and list updates inside read models.
- **`03-flight-booking`**: Distributed travel booking domain testing stateful `process-manager` sagas, compensating commands, and scheduled `ProcessTimer` timeouts.
- **`04-virtual-apartment`**: Multi-context property & tenancy domain demonstrating subdomains, `shared-kernel` crates, Dynamic Consistency Boundaries (`consults`), entities, value objects, domain services, policies, event handlers, and process managers.
- **`05-helpdesk-ticketing`**: Support ticketing domain exercising Dynamic Consistency Boundaries, actor authorization, entities, and domain services.
- **`06-notification-delivery`**: Multi-context notification domain demonstrating `context-mapping` patterns, `external-system` infrastructure traits, policies, and outbound event handlers.
- **`07-life-chart`**: Desktop mood & symptom tracking application (Life Chart Method) demonstrating Tauri v2 + Svelte 5 (`@chromatofoor/ui`) presentation bindings.
- **`08-digital-identity-ecosystem`**: Multi-context Decentralized Identity (OID4VCI, OID4VP, OpenID Federation, Token Status List) reference architecture demonstrating declarative `model.infra.esdm.yaml` HTTP specification bindings, cross-context policies, and cryptographic domain boundaries.

## Regenerating & Verifying Testbeds

To lint, test, regenerate all testbeds, and verify zero git-diff non-determinism in one step from the repository root:

```bash
./scripts/verify_testbeds.sh
```

To regenerate an individual testbed manually:

```bash
cargo run -p chromatofoor-gen -- \
  --input testbed/<example>/model.esdm.yaml \
  --features testbed/<example>/model.feature.esdm.yaml \
  --lifecycle testbed/<example>/model.lifecycle.esdm.yaml \
  --out-dir testbed/<example>/workspace \
  --chromatofoor-path ../../../../chromatofoor
```
*(Add `--infra testbed/<example>/model.infra.esdm.yaml` when an infrastructure binding file is present.)*
