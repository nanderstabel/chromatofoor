# Chromatofoor Testbeds

This directory contains five progressive, domain-agnostic DDD reference domains used to verify `chromatofoor`, `chromatofoor-derive`, `chromatofoor-gen`, `esdm`, and all presentation adapters under `./scripts/verify_testbeds.sh`. Together, `01` through `05` cover 100% of ESDM's 18 core `kind`s plus `aggregate-lifecycle`, `feature`, and all five `presentation-binding` transports.

Each subdirectory contains declarative `.esdm.yaml` specifications (`model.esdm.yaml`, `model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and optional `model.infra.esdm.yaml`) alongside its generated Cargo `workspace/`.

## Canonical Progression (`01` → `05`)

| # | Directory | Classic DDD Problem Space | Primary ESDM & Runtime Mechanisms Covered |
| :--- | :--- | :--- | :--- |
| **01** | [`01-bank-account`](./01-bank-account/model.esdm.yaml) | Retail bank account | `domain`, `bounded-context`, `actor` (`human`), `aggregate` (`generated(uuid)`), `aggregate-lifecycle` (`created`), `command`, `event`, `read-model` (`DocumentView`), `query`, `feature` (`then.events`, `events: []` idempotency, `rejection.invariant`, unauthorized actor rejection, `then.result`). |
| **02** | [`02-ecommerce-checkout`](./02-ecommerce-checkout/model.esdm.yaml) | Shopping cart & checkout | `entity` (`identifiedBy: schema`), `value-object`, `$ref` schema composition, `validator::Validate` nested & range constraints, multi-event state invariants (`empty-cart`, `cart-already-checked-out`). |
| **03** | [`03-course-enrollment`](./03-course-enrollment/model.esdm.yaml) | University course registration | `dynamic-consistency-boundary` (`identifiedBy: command-payload` composite key + cross-aggregate `consults` + free-standing BC events), `domain-service`, DCB `feature` scenarios. |
| **04** | [`04-flight-booking`](./04-flight-booking/model.esdm.yaml) | Travel reservation saga | Multi-BC coordination (`flight`, `hotel`, `payment`), `actor` (`system`), `process-manager` (`correlatedBy: event-field`, `startsWhen`, `endsWhen`, `timers` with `after`, `setTimers`, `cancelTimers`, compensating commands, `deliveryGuarantee: at-least-once`, `idempotency`), `customer-supplier` `context-mapping`, PM `feature` scenarios. |
| **05** | [`05-cargo-shipping`](./05-cargo-shipping/model.esdm.yaml) | Freight & cargo routing (Evans) | `subdomain` (`core`, `supporting`, `generic`), `shared-kernel` (`tier: shared-kernel`), `policy` (`React`), `event-handler` (`Handle` + outbound `sideEffects`), `external-system` (`outbound`), `anti-corruption-layer` `context-mapping` (`src/infrastructure/`), `model.infra.esdm.yaml` (`http-rest`, `graphql`, `mcp`, `cli`, `tauri-ipc`), `typescript-client/`. |

## Regenerating & Verifying Testbeds

To verify strict Green-phase completion (all generated tests passing and zero leftover `todo!` stubs) and zero git-diff non-determinism from the repository root:

```bash
./scripts/verify_testbeds.sh
```

When testbeds are freshly scaffolded in Red state (before manual `todo!(...)` stubs are implemented), verify compilation, lint cleanliness (`-D warnings -A clippy::todo`), and regeneration determinism with:

```bash
./scripts/verify_testbeds.sh --red-phase
```

To regenerate an individual testbed manually (`model.lifecycle.esdm.yaml`, `model.feature.esdm.yaml`, and `model.infra.esdm.yaml` are auto-discovered alongside `model.esdm.yaml`):

```bash
cargo run -p chromatofoor-gen --features tui -- \
  --input testbed/04-flight-booking/model.esdm.yaml \
  --out-dir testbed/04-flight-booking/workspace \
  --chromatofoor-path ../../../../chromatofoor
```
*(Use `--features full` instead of `--features tui` when generating `05-cargo-shipping` so all presentation servers and `typescript-client/` are emitted.)*
