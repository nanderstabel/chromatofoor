# Chromatofoor Core Runtime (`chromatofoor`)

## Overview
`chromatofoor` is the zero-HTTP-dependency Domain-Driven Design (DDD), CQRS, and Event Sourcing runtime crate built on `architecturekit`. It re-exports all 18 `chromatofoor-derive` procedural attribute macros alongside foundational traits and execution engines for strategic DDD (`Domain`, `Subdomain`, `BoundedContext`, `ContextMapping`, `ExternalSystem`), tactical consistency boundaries (`Aggregate`, `DynamicConsistencyBoundary`, `Command`, `Event`, `ReadModel`, `DocumentView`, `Query`, `Actor`, `Authorized`), resumable cross-context integrations (`EventConsumer`, `Policy`, `EventHandler`, `ProcessManager`, `ProcessTimer`), schema-rich runtime composition (`DomainBuilder`, `DomainRuntime`, `Presentation`), and in-memory Given-When-Then specification harnesses (`chromatofoor::testing`).

## Interface
- **Crate Root Re-Exports & Helpers** (`src/lib.rs`):
  - Re-exports all 18 `chromatofoor-derive` attribute macros (`actor`, `aggregate`, `bounded_context`, `command`, `context_mapping`, `domain`, `domain_service`, `dynamic_consistency_boundary`, `entity`, `event`, `event_handler`, `external_system`, `policy`, `process_manager`, `query`, `read_model`, `subdomain`, `value_object`).
  - Re-exports `common::{Identifier, Problem, ProblemDetails, SubjectSegment, SubjectSegmentError}`, `subject_for<A: Kind>(id) -> architecturekit::Subject`, and foundational crates (`architecturekit`, `chrono`, `constcat`, `serde`, `serde_json`, `tracing`, `uuid`, `validator`).
- **Common Identity, Problem Details, Scope & Topic Primitives** (`src/common/`):
  - `Kind` (`src/common/kind.rs`) — Core static metadata trait (`Scope`, `KIND`, `NAME`, `TOPIC`, `DESCRIPTION`, `topic(&self)`).
  - `ProblemDetails`, `Problem` (`src/common/problem.rs`) — Zero-HTTP-dependency RFC 9457 Problem Details envelope (`type`, `title`, `status`, `detail`, `instance`, `code`, and flattened `extensions`) and `Problem` conversion trait.
  - `Scope<P>`, `Topic`, `topic_in_domain` (`src/common/scope.rs`, `src/common/topic.rs`) — Hierarchical scope phantom markers and `const fn` prefix verification.
  - `SubjectSegment`, `SubjectSegmentError`, `Identifier`, `IdSource`, `Identified`, `FieldAsIdStr`, `NonIdField`, `ValidIdField`, `NonValidIdField`, `IdFieldProbe`, `const_str_eq`, `validate_command_id_fields`, `resolve_subject` (`src/common/identified.rs`) — Validated subject segments (`^[A-Za-z0-9_-]+$`), compile-time ID field probing/specialization helpers, and runtime subject resolution.
- **Strategic Domain Traits** (`src/domain.rs`):
  - `Domain`, `Subdomain`, `SubdomainType` (`Core`, `Supporting`, `Generic`), `BoundedContextInDomain`, `assert_bounded_context_in_domain`, plus re-exports of `ConsumerDescriptor`, `DispatchError`, `DomainBuilder`, `DomainRuntime`, `Presentation`, `Problem`, and `ProblemDetails`.
- **Tactical Bounded-Context Traits & Views** (`src/bounded_context/`):
  - `BoundedContext`, plus re-exports of `CommandDescriptor`, `QueryDescriptor`, and `ReadModelDescriptor` (`mod.rs`).
  - `Actor`, `ActorType` (`Human`, `System`), `Authorized` (`actor.rs`).
  - `Aggregate` (`aggregate.rs`), `DynamicConsistencyBoundary` (`dynamic_consistency_boundary.rs`), `Command<S>`, `CommandScope`, `CommandExecutionError<E>`, `PreconditionFailure` (`command.rs`), `Event<S>` (`event.rs`).
  - `Entity` (`entity.rs`), `ValueObject` (`value_object.rs`), `DomainService` (`domain_service.rs`).
  - `ReadModel`, `Project<E>`, `DocumentView<T>` (`new`, `inner`, `subject_id`, `seen`, `insert`, `upsert`, `upsert_subject`, `update`, `delete`, `get`, `all`, `latest`, `Revisioned`, and `Serialize` returning the latest document or `T::default()` in `read_model.rs`), `Query<R>` (`query.rs`).
- **Cross-Context Integration & Process Orchestration** (`src/integration/`):
  - `ContextMapping`, `ContextMappingType`, `MappingEndpoint` (`context_mapping.rs`), `ExternalSystem`, `ExternalDirection` (`external_system.rs`).
  - `EventConsumer`, `DeliveryGuarantee` (`AtLeastOnce`, `AtMostOnce`), `Idempotency`, `IdempotencyOwner` (`event_consumer.rs`).
  - `EventHandler`, `Handle<E>` (`event_handler.rs`), `Policy`, `React<E>`, `execute_policy_command` (`policy.rs`).
  - `ProcessManager`, `ProcessCommand`, `OnEvent<E>`, `OnTimer<T>`, `ProcessInstanceSaved`, `ProcessInstanceEvent`, `ProcessInstanceState`, `ProcessTimerFired`, `load_process_instance`, `save_process_instance`, `flush_process_commands`, `execute_process_command` (`process_manager.rs`).
  - `ProcessTimer`, `SetTimer<T>`, `CancelTimer<T>`, `TimerSchedule`, `TimerUnit`, `AsTimerTimestamp`, `start_process_timers`, `supervise_process_timers` (`timer_scheduler.rs`).
- **Runtime Composition & Transport Hosting** (`src/runtime/`):
  - `DomainBuilder<D>` (`new`, `register_command`, `register_read_model`, `register_query`, `register_consumer`, `attach`, `build`, `serve` in `domain.rs`).
  - `DomainRuntime` (`domain_name`, `domain_topic`, `domain_description`, `store`, `commands`, `queries`, `read_models`, `consumers`, `actors`, `commands_for_actor`, `queries_for_actor`, `catalog_json`, `attach`, `attached_presentations`, `serve`, `wait_caught_up`, `dispatch_command`, `dispatch_query`, `subscribe_events` in `domain.rs`).
  - `Presentation` (`name`, `serve` in `domain.rs`).
  - `CommandDescriptor`, `QueryDescriptor`, `ReadModelDescriptor`, `ConsumerDescriptor` (`descriptor.rs`).
  - `DispatchError` (`BoundedContextNotFound`, `CommandNotFound`, `QueryNotFound`, `Unauthorized`, `InvalidPayload`, `Execution(Box<ProblemDetails>)`, `to_problem` in `error.rs`).
- **Given-When-Then Specification Harness** (`src/testing.rs`):
  - `given<S>`, `UnitFixture`, `UnitDecision` — Consistency-unit tests (`when`, `when_as`, `when_by`, `then_events`, `then_nothing`, `then_rejected`, `then_rejected_reason`, `then_state`, `then_preconditions`).
  - `given_process_manager<P>`, `ProcessFixture`, `ProcessDecision` — Stateful saga and timer tests (`when_event`, `when_timer`, `when_timer_named`, `then_emits`, `then_emits_command`, `then_set_timer`, `then_set_timers`, `then_cancel_timer`, `then_cancel_timers`, `then_state`, `then_state_json`, `then_ended`).
  - `given_read_model<R>`, `given_read_model_with_id<R>`, `ReadModelFixture`, `ReadModelDecision`, `assert_json_subset` — Read-model projection and query tests (`when`, `when_as`, `when_by`, `then_result`, `then_result_json`, `then_read_model`, `then_read_model_json`, `then_read_model_with`, `then_rejected_reason`).

## Invariants
- **Strict Rust Safety & Lint Baseline** (`Cargo.toml`, `src/testing.rs`, `src/common/identified.rs`): Inherits workspace `#![deny(unsafe_code)]`, `clippy::pedantic`, and `deny(unwrap_used, expect_used, panic, todo, unimplemented)`. `#![allow(clippy::panic)]` is scoped to `src/testing.rs` assertion helpers, while `Identifier`'s `From<&str | String | &String | Uuid>` impls carry targeted `#[allow(clippy::panic)]` and panic at runtime if given an invalid segment (fallible callers must use `Identifier::try_new`, `FromStr`, or `Deserialize`).
- **Subject Segment & Scope Hierarchy Guarantees** (`src/common/`, `src/domain.rs`, `src/bounded_context/`, `src/runtime/domain.rs`):
  - `SubjectSegment` rejects empty strings and any character outside ASCII `[A-Za-z0-9_-]`.
  - Compile-time witness bounds enforce `BoundedContext: Kind<Scope: Domain>`, `EventInBoundedContext<BC>` on DCBs and `ReadModel`s, `Query<R>: Kind<Scope = R::Scope>`, `ActorInBoundedContext<BC>`, and `const { assert!(topic_in_domain(D::TOPIC, ...)) }` across all `DomainBuilder<D>` registrations (with runtime assertions preventing duplicate registrations).
- **Two-Phase Fail-Closed Actor Authorization** (`src/bounded_context/actor.rs`, `src/runtime/error.rs`, `src/runtime/descriptor.rs`):
  - Empty `Authorized::ALLOWED_ACTORS` denies all callers by default.
  - Phase 1 (`ensure_authorized_type`) runs **before** `serde_json::from_value`, rejecting `actor = None` or unauthorized actors with `DispatchError::Unauthorized` prior to parsing untrusted JSON payloads.
  - Phase 2 (`ensure_authorized_instance`) runs **after** deserialization via `instance.is_authorized_for(actor)` to enforce variant-specific `#[actors(...)]` overrides on command enums before execution.
- **Pre-Decide Validation, Zero-Event Precondition Parity & DCB Optimistic Concurrency** (`src/bounded_context/command.rs`, `src/bounded_context/aggregate.rs`, `src/bounded_context/dynamic_consistency_boundary.rs`): `Command::decider()` and `DynamicConsistencyBoundary::execute` always run `validate_payload()` before `decide`; when `decide` returns zero events, `Aggregate::execute` and `DynamicConsistencyBoundary::execute` still verify explicit database preconditions (`isSubjectPopulated`, `isSubjectPristine`, `isSubjectOnEventId`) via `verify_zero_event_preconditions` so live execution matches `chromatofoor::testing`; DCB non-empty writes guard `<Self::Scope as Kind>::TOPIC` with recursive `is_subject_on_event_id` (or `is_subject_pristine` when empty) preconditions.
- **Resumable Checkpoints, Outbox Progress & Timer Safety** (`src/integration/`):
  - `EventConsumer` persists checkpoints at `/checkpoints/<kind>/<domain>/<consumer>`: `AtMostOnce` commits the checkpoint before `handle` and logs a warning without failing on handler errors, whereas `AtLeastOnce` propagates handler errors and commits the checkpoint only after `handle` succeeds.
  - Downstream decider rejections (`architecturekit::Error::Decider`) and zero-event precondition failures (`CommandExecutionError::Precondition`) in `execute_policy_command` and `execute_process_command` are logged and isolated so business/missing-target rejections never stall consumer streams, while transient/conflict errors propagate for retry.
  - `ProcessManager` snapshots and `pending_commands` outboxes persist at `/process-managers/<domain>/<pm>/<correlation_id>` under `Precondition::on_state_read()`; duplicate/older `last_event_id` and post-`ended` writes become no-ops unless draining remaining `pending_commands`, and terminal `is_ended()` clears all `active_timers`.
  - `ProcessTimerWatcher` reloads `ProcessInstanceState` before writing `ProcessTimerFired` to `/<domain>/<pm>/timers/<correlation_id>`, verifying `!current.ended` and matching `active_timers` deadlines, retrying transient failures up to 16 times (`50ms`..`5s` backoff), and aborting immediately on `Permanent` errors (`TimerUnit::add_to` saturates at `DateTime::<Utc>::MAX_UTC`).

## Side Effects
- **Store Persistence & Projections** (`src/bounded_context/`, `src/integration/`, `src/runtime/`): Reads and appends events, checkpoints (`/checkpoints/...`), process-manager snapshots/outboxes (`/process-managers/...`), and timer events (`/<domain>/<pm>/timers/...`) via `architecturekit::Store`; `DomainBuilder::build` starts background `ReadModel` and `EventConsumer` projections, awaits initial `architecturekit::wait_caught_up`, and cancels supervised projections on final `DomainRuntime` drop.
- **Async Tasks, Channels & Timers** (`src/integration/timer_scheduler.rs`, `src/runtime/domain.rs`): Spawns Tokio timer tasks (`tokio::time::sleep`), awaits a single attached `Presentation` directly or runs multiple `Presentation`s concurrently on a `tokio::task::JoinSet` (aborting peers on first error or panic), and streams raw store events over unbounded MPSC channels via `DomainRuntime::subscribe_events()`.
- **Entropy & Synchronized Views** (`src/common/identified.rs`, `src/bounded_context/read_model.rs`, `src/testing.rs`): `resolve_subject` invokes OS CSPRNG (`Uuid::new_v4()`) for generated IDs without an explicit `"id"` field; `DocumentView` mutates synchronized `architecturekit::InMemoryView` state; `chromatofoor::testing` runs purely in-memory.

## Verification
```bash
cargo test -p chromatofoor --all-features
cargo clippy -p chromatofoor --all-targets --all-features -- -D warnings
```
