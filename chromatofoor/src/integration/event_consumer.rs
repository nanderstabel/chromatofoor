//! Shared event-consumption, delivery-guarantee, and runtime subscription building blocks
//! for domain-scoped integration components ([`Policy`](crate::integration::Policy),
//! [`EventHandler`](crate::integration::EventHandler), and [`ProcessManager`](crate::integration::ProcessManager)).

use std::{future::Future, sync::Arc};

use crate::{common::Kind, domain::Domain};

/// Delivery guarantee for event-consuming integration components ([`Policy`](crate::integration::Policy),
/// [`EventHandler`](crate::integration::EventHandler), [`ProcessManager`](crate::integration::ProcessManager)).
///
/// Both guarantees persist a [`Revision`](architecturekit::Revision) checkpoint in the
/// [`Store`](architecturekit::Store) under `/checkpoints/<domain>/<consumer>` so normal
/// restarts resume where the consumer left off. They differ in **when** that checkpoint is
/// written relative to [`EventConsumer::handle`]:
///
/// - [`DeliveryGuarantee::AtLeastOnce`]: saves the checkpoint **after** `handle` succeeds.
///   If `handle` fails or the process crashes mid-flight, restarting the consumer re-delivers
///   the event. Requires an explicit [`Idempotency`] declaration.
/// - [`DeliveryGuarantee::AtMostOnce`]: saves the checkpoint **before** `handle` runs.
///   Even if `handle` fails or crashes, restarting the consumer skips the event and never
///   delivers it a second time.
///
/// # Examples
///
/// ## `AtLeastOnce`: re-delivers on restart if `handle` failed before checkpointing
///
/// ```rust
/// # use std::sync::{Arc, atomic::{AtomicUsize, Ordering as AtomicOrdering}};
/// # use chromatofoor::{
/// #     architecturekit::{
/// #         CancellationToken, DomainError, Envelope, EventOn, InMemoryEventStore,
/// #         Precondition, Store, Subject, wait_caught_up,
/// #     },
/// #     integration::{DeliveryGuarantee, EventConsumer, Idempotency, IdempotencyOwner},
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced { pub order_id: String }
/// # #[derive(Clone, Debug, PartialEq, chromatofoor::architecturekit::Events)]
/// # pub enum ConsumerEvents { OrderPlaced(OrderPlaced) }
/// #[derive(Clone, Default)]
/// pub struct AtLeastOnceConsumer {
///     pub attempts: Arc<AtomicUsize>,
/// }
/// # impl chromatofoor::common::Kind for AtLeastOnceConsumer {
/// #     type Scope = Commerce;
/// #     const KIND: &'static str = "event-handler";
/// #     const NAME: &'static str = "at-least-once-consumer";
/// #     const TOPIC: &'static str = "/commerce/at-least-once-consumer";
/// # }
///
/// impl EventConsumer for AtLeastOnceConsumer {
///     type Event = ConsumerEvents;
///     type Error = DomainError;
///
///     const DELIVERY_GUARANTEE: DeliveryGuarantee = DeliveryGuarantee::AtLeastOnce;
///     const IDEMPOTENCY: Option<Idempotency> = Some(Idempotency {
///         owner: IdempotencyOwner::SelfManaged,
///         strategy: None,
///     });
///
///     async fn handle(&self, _store: &Store, _event: Envelope<Self::Event>) -> Result<(), Self::Error> {
///         if self.attempts.fetch_add(1, AtomicOrdering::SeqCst) == 0 {
///             return Err(DomainError::new("boom"));
///         }
///         Ok(())
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// # store.write(
/// #     vec![EventOn::new(Subject::new("/commerce/ordering"), &OrderPlaced { order_id: "ord-1".to_owned() })],
/// #     vec![Precondition::unconditionally()],
/// # ).await.unwrap();
/// let consumer = AtLeastOnceConsumer::default();
///
/// // 1st run fails in `handle` BEFORE the checkpoint is saved:
/// let run1 = consumer.clone().start(Arc::clone(&store), CancellationToken::new());
/// assert!(wait_caught_up(&[&run1]).await.is_err());
/// assert_eq!(consumer.attempts.load(AtomicOrdering::SeqCst), 1);
///
/// // Restarting re-delivers the uncommitted event (attempts == 2) and saves the checkpoint:
/// let token2 = CancellationToken::new();
/// let run2 = consumer.clone().start(Arc::clone(&store), token2.clone());
/// wait_caught_up(&[&run2]).await.unwrap();
/// token2.cancel();
/// assert_eq!(consumer.attempts.load(AtomicOrdering::SeqCst), 2);
///
/// // Restarting again after a saved checkpoint does NOT re-deliver (attempts stays 2):
/// let token3 = CancellationToken::new();
/// let run3 = consumer.clone().start(Arc::clone(&store), token3.clone());
/// wait_caught_up(&[&run3]).await.unwrap();
/// token3.cancel();
/// assert_eq!(consumer.attempts.load(AtomicOrdering::SeqCst), 2);
/// # });
/// ```
///
/// ## `AtMostOnce`: saves checkpoint first, so a failed `handle` is never re-delivered on restart
///
/// ```rust
/// # use std::sync::{Arc, atomic::{AtomicUsize, Ordering as AtomicOrdering}};
/// # use chromatofoor::{
/// #     architecturekit::{
/// #         CancellationToken, DomainError, Envelope, EventOn, InMemoryEventStore,
/// #         Precondition, Store, Subject, wait_caught_up,
/// #     },
/// #     integration::{DeliveryGuarantee, EventConsumer},
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced { pub order_id: String }
/// # #[derive(Clone, Debug, PartialEq, chromatofoor::architecturekit::Events)]
/// # pub enum ConsumerEvents { OrderPlaced(OrderPlaced) }
/// #[derive(Clone, Default)]
/// pub struct AtMostOnceConsumer {
///     pub attempts: Arc<AtomicUsize>,
/// }
/// # impl chromatofoor::common::Kind for AtMostOnceConsumer {
/// #     type Scope = Commerce;
/// #     const KIND: &'static str = "event-handler";
/// #     const NAME: &'static str = "at-most-once-consumer";
/// #     const TOPIC: &'static str = "/commerce/at-most-once-consumer";
/// # }
///
/// impl EventConsumer for AtMostOnceConsumer {
///     type Event = ConsumerEvents;
///     type Error = DomainError;
///
///     const DELIVERY_GUARANTEE: DeliveryGuarantee = DeliveryGuarantee::AtMostOnce;
///
///     async fn handle(&self, _store: &Store, _event: Envelope<Self::Event>) -> Result<(), Self::Error> {
///         self.attempts.fetch_add(1, AtomicOrdering::SeqCst);
///         Err(DomainError::new("boom"))
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// # store.write(
/// #     vec![EventOn::new(Subject::new("/commerce/ordering"), &OrderPlaced { order_id: "ord-1".to_owned() })],
/// #     vec![Precondition::unconditionally()],
/// # ).await.unwrap();
/// let consumer = AtMostOnceConsumer::default();
///
/// // 1st run saves checkpoint BEFORE `handle`; when `handle` fails, the failure is logged
/// // without retrying the already-committed event:
/// let token1 = CancellationToken::new();
/// let run1 = consumer.clone().start(Arc::clone(&store), token1.clone());
/// wait_caught_up(&[&run1]).await.unwrap();
/// token1.cancel();
/// assert_eq!(consumer.attempts.load(AtomicOrdering::SeqCst), 1);
///
/// // Restarting skips the event because its checkpoint was already saved (attempts stays 1):
/// let token2 = CancellationToken::new();
/// let run2 = consumer.clone().start(Arc::clone(&store), token2.clone());
/// wait_caught_up(&[&run2]).await.unwrap();
/// token2.cancel();
/// assert_eq!(consumer.attempts.load(AtomicOrdering::SeqCst), 1);
/// # });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// Events are delivered at least once; duplicate deliveries may occur and
    /// require an explicit [`Idempotency`] declaration.
    AtLeastOnce,
    /// Events are delivered at most once; duplicate deliveries do not occur.
    AtMostOnce,
}

/// Who is responsible for tolerating duplicate event delivery under
/// [`DeliveryGuarantee::AtLeastOnce`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdempotencyOwner {
    /// The consuming component deduplicates deliveries itself (e.g. tracking
    /// processed [`Envelope::id`](architecturekit::Envelope::id)s or conditional writes).
    SelfManaged,
    /// The downstream target (aggregate command or external system API via an
    /// idempotency key) handles deduplication.
    Downstream,
    /// The messaging/storage infrastructure guarantees deduplication (e.g. a
    /// transactional outbox or broker-level deduplication).
    Infrastructure,
    /// No deduplication is performed (deliberate opt-out when occasional
    /// duplicates on crash recovery are acceptable).
    None,
    /// The reaction is naturally idempotent without explicit deduplication
    /// (e.g. pure set/overwrite operations).
    NotRequired,
}

/// Idempotency configuration paired with [`DeliveryGuarantee`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Idempotency {
    /// Which layer owns duplicate-delivery protection.
    pub owner: IdempotencyOwner,
    /// Optional description of the concrete deduplication strategy.
    pub strategy: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, architecturekit::Event)]
#[event(type = "io.esdm.checkpoint.saved")]
struct CheckpointSaved {
    revision: u64,
}

#[derive(Clone, Debug, PartialEq, architecturekit::Events)]
enum CheckpointEvent {
    Saved(CheckpointSaved),
}

#[derive(Clone, Default)]
struct CheckpointState {
    revision: Option<architecturekit::Revision>,
}

impl architecturekit::State for CheckpointState {
    type Event = CheckpointEvent;

    fn evolve(self, event: &Self::Event) -> Self {
        match event {
            CheckpointEvent::Saved(saved) => Self {
                revision: architecturekit::Revision::try_from(saved.revision).ok(),
            },
        }
    }

    fn from_latest() -> Option<&'static str> {
        Some(<CheckpointSaved as architecturekit::Event>::EVENT_TYPE)
    }
}

/// Base trait for domain-scoped integration components ([`Policy`](crate::integration::Policy),
/// [`EventHandler`](crate::integration::EventHandler), [`ProcessManager`](crate::integration::ProcessManager)) that consume events across their [`Domain`]'s subject tree.
pub trait EventConsumer: Kind<Scope: Domain> + Send + Sync + 'static {
    /// The generated sum type of all events handled by this consumer.
    type Event: architecturekit::Events + Send + Sync + 'static;
    /// The error type returned when handling an event fails.
    type Error: std::error::Error + architecturekit::Categorized + Send + Sync + 'static;

    /// The delivery guarantee configured for this consumer.
    const DELIVERY_GUARANTEE: DeliveryGuarantee;
    /// Optional idempotency metadata (required when [`Self::DELIVERY_GUARANTEE`] is [`DeliveryGuarantee::AtLeastOnce`]).
    const IDEMPOTENCY: Option<Idempotency> = None;

    /// Handles a single incoming event envelope against `store`.
    fn handle(
        &self,
        store: &architecturekit::Store,
        event: architecturekit::Envelope<Self::Event>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Optional hook invoked by [`EventConsumer::start`] to launch auxiliary background tasks
    /// (such as [`ProcessManager`](crate::integration::ProcessManager) timer schedulers)
    /// bound to the same `store` and `token`.
    fn on_start(
        &self,
        _store: &Arc<architecturekit::Store>,
        _token: &architecturekit::CancellationToken,
    ) {
    }

    /// Subscribes this consumer to its domain's subject tree on `store`,
    /// resuming from its persisted checkpoint under `/checkpoints/<kind>/<domain>/<consumer>`.
    #[must_use]
    fn start(
        self,
        store: Arc<architecturekit::Store>,
        token: architecturekit::CancellationToken,
    ) -> architecturekit::ProjectionRun
    where
        Self: Sized,
    {
        self.on_start(&store, &token);
        let checkpoint_subject =
            architecturekit::Subject::new(&format!("/checkpoints/{}{}", Self::KIND, Self::TOPIC));
        let runner = Arc::new(DomainProjection {
            consumer: self,
            store: Arc::clone(&store),
            checkpoint_subject,
        });
        store.start_resumable_projection(
            architecturekit::Subjects::tree(architecturekit::Subject::new(
                <Self::Scope as Kind>::TOPIC,
            )),
            runner,
            token,
            architecturekit::ProjectionOptions::named(format!("{}:{}", Self::KIND, Self::TOPIC)),
        )
    }
}

struct DomainProjection<C> {
    consumer: C,
    store: Arc<architecturekit::Store>,
    checkpoint_subject: architecturekit::Subject,
}

impl<C: EventConsumer> DomainProjection<C> {
    async fn write_checkpoint(
        &self,
        event_id: architecturekit::Revision,
    ) -> Result<(), architecturekit::Error<C::Error>> {
        self.store
            .write(
                vec![architecturekit::EventOn::new(
                    self.checkpoint_subject.clone(),
                    &CheckpointSaved {
                        revision: event_id.get(),
                    },
                )],
                vec![architecturekit::Precondition::unconditionally()],
            )
            .await?;
        Ok(())
    }
}

impl<C: EventConsumer> architecturekit::Projection for DomainProjection<C> {
    type Event = C::Event;
    type Error = architecturekit::Error<C::Error>;

    async fn apply(
        &self,
        event: architecturekit::Envelope<Self::Event>,
    ) -> Result<(), Self::Error> {
        let event_id = event.id;
        if C::DELIVERY_GUARANTEE == DeliveryGuarantee::AtMostOnce {
            self.write_checkpoint(event_id).await?;
            if let Err(err) = self.consumer.handle(&self.store, event).await {
                tracing::warn!(
                    consumer = C::TOPIC,
                    event_id = event_id.get(),
                    error = %err,
                    "at-most-once consumer failed after checkpoint commit; skipping retry"
                );
            }
            return Ok(());
        }
        self.consumer
            .handle(&self.store, event)
            .await
            .map_err(architecturekit::Error::Decider)
    }
}

impl<C: EventConsumer> architecturekit::ResumableProjection for DomainProjection<C> {
    async fn checkpoint(&self) -> Result<Option<architecturekit::Revision>, Self::Error> {
        let state: CheckpointState = self.store.load(&self.checkpoint_subject).await?;
        Ok(state.revision)
    }

    async fn save_checkpoint(
        &self,
        event_id: architecturekit::Revision,
    ) -> Result<(), Self::Error> {
        if C::DELIVERY_GUARANTEE == DeliveryGuarantee::AtLeastOnce {
            self.write_checkpoint(event_id).await?;
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn map_error<E: std::fmt::Debug>(
    err: architecturekit::Error<E>,
) -> architecturekit::Error<architecturekit::DomainError> {
    match err {
        architecturekit::Error::Decider(e) => {
            architecturekit::Error::Decider(architecturekit::DomainError::new(format!("{e:?}")))
        }
        architecturekit::Error::Transient(t) => architecturekit::Error::Transient(t),
        architecturekit::Error::Permanent(p) => architecturekit::Error::Permanent(p),
        architecturekit::Error::OutcomeUnknown(f) => architecturekit::Error::OutcomeUnknown(f),
        architecturekit::Error::EmptyRange(r) => architecturekit::Error::EmptyRange(r),
        #[allow(unreachable_patterns)]
        other => {
            architecturekit::Error::Decider(architecturekit::DomainError::new(format!("{other:?}")))
        }
    }
}

pub(crate) async fn execute_isolated_command<S, C>(
    store: &architecturekit::Store,
    command: C,
    consumer_kind: &'static str,
) -> Result<
    Vec<architecturekit::eventsourcingdb::Event>,
    architecturekit::Error<architecturekit::DomainError>,
>
where
    S: crate::bounded_context::command::CommandScope,
    C: crate::bounded_context::Command<S>,
{
    match S::execute(store, command).await {
        Ok(events) => Ok(events),
        Err(crate::bounded_context::CommandExecutionError::Store(
            architecturekit::Error::Decider(err),
        )) => {
            tracing::warn!(
                command = C::TOPIC,
                consumer = consumer_kind,
                error = %err,
                "command rejected by target decider; isolating stream"
            );
            Ok(Vec::new())
        }
        Err(crate::bounded_context::CommandExecutionError::Precondition(failure)) => {
            tracing::warn!(
                command = C::TOPIC,
                consumer = consumer_kind,
                error = %failure,
                "command precondition failed on zero-event decision; isolating stream"
            );
            Ok(Vec::new())
        }
        Err(crate::bounded_context::CommandExecutionError::Store(
            architecturekit::Error::Transient(t),
        )) => Err(architecturekit::Error::Transient(t)),
        Err(crate::bounded_context::CommandExecutionError::Store(
            architecturekit::Error::Permanent(p),
        )) => Err(architecturekit::Error::Permanent(p)),
        Err(crate::bounded_context::CommandExecutionError::Store(
            architecturekit::Error::OutcomeUnknown(f),
        )) => Err(architecturekit::Error::OutcomeUnknown(f)),
        Err(crate::bounded_context::CommandExecutionError::Store(
            architecturekit::Error::EmptyRange(r),
        )) => Err(architecturekit::Error::EmptyRange(r)),
        #[allow(unreachable_patterns)]
        Err(other) => Err(architecturekit::Error::Decider(
            architecturekit::DomainError::new(format!("{other}")),
        )),
    }
}
