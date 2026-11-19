//! Stateful, correlated multi-event coordination (`kind: process-manager`).

use std::{collections::BTreeMap, future::Future};

use crate::integration::{EventConsumer, ProcessTimer};

/// Internal snapshot event persisted to `/process-managers/<domain>/<pm>/<correlation_id>`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, architecturekit::Event)]
#[event(type = "io.esdm.process-manager.saved")]
pub struct ProcessInstanceSaved {
    /// Whether this correlation key has been initialized by a `starts_when` event.
    pub started: bool,
    /// Whether this instance has reached an `ends_when` terminal state.
    pub ended: bool,
    /// Revision of the last event processed by this instance (for self-managed idempotency).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<u64>,
    /// Currently armed timers (`timers[].name` -> target UTC deadline) for this instance.
    pub active_timers: BTreeMap<String, chrono::DateTime<chrono::Utc>>,
    /// Serialized domain commands queued in this instance's outbox awaiting execution.
    #[serde(default)]
    pub pending_commands: Vec<serde_json::Value>,
    /// Serialized per-instance state.
    pub state: serde_json::Value,
}

/// Event sum type for [`ProcessInstanceState`].
#[derive(Clone, Debug, PartialEq, architecturekit::Events)]
pub enum ProcessInstanceEvent {
    /// Snapshot of a process manager instance.
    Saved(ProcessInstanceSaved),
}

/// Stored metadata and state for a single correlated [`ProcessManager`] instance.
#[derive(Clone, Debug, Default)]
pub struct ProcessInstanceState {
    /// Whether this correlation key has been initialized by a `starts_when` event.
    pub started: bool,
    /// Whether this instance has reached an `ends_when` terminal state.
    pub ended: bool,
    /// Revision of the last event processed by this instance.
    pub last_event_id: Option<u64>,
    /// Currently armed timers (`timers[].name` -> target UTC deadline) for this instance.
    pub active_timers: BTreeMap<String, chrono::DateTime<chrono::Utc>>,
    /// Serialized domain commands queued in this instance's outbox awaiting execution.
    pub pending_commands: Vec<serde_json::Value>,
    /// Serialized per-instance state.
    pub state: serde_json::Value,
}

impl architecturekit::State for ProcessInstanceState {
    type Event = ProcessInstanceEvent;

    fn evolve(self, event: &Self::Event) -> Self {
        match event {
            ProcessInstanceEvent::Saved(saved) => Self {
                started: saved.started,
                ended: saved.ended,
                last_event_id: saved.last_event_id,
                active_timers: saved.active_timers.clone(),
                pending_commands: saved.pending_commands.clone(),
                state: saved.state.clone(),
            },
        }
    }

    fn from_latest() -> Option<&'static str> {
        Some(<ProcessInstanceSaved as architecturekit::Event>::EVENT_TYPE)
    }
}

/// Internal event written to `/<domain>/<pm>/timers/<correlation_id>` when a [`ProcessTimer`] fires.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, architecturekit::Event)]
#[event(type = "io.esdm.process-manager.timer-fired")]
pub struct ProcessTimerFired {
    /// Topic of the target [`ProcessManager`].
    pub process_manager: String,
    /// Correlation key of the target instance.
    pub correlation_id: String,
    /// Kebab-case timer name (`ProcessTimer::NAME`).
    pub timer: String,
    /// Target UTC deadline of the timer instance that fired, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

/// Loads the stored [`ProcessInstanceState`] and decoded instance `P` (if started) for `correlation_id`.
pub async fn load_process_instance<P: ProcessManager>(
    store: &architecturekit::Store,
    correlation_id: &str,
) -> Result<(ProcessInstanceState, Option<P>), architecturekit::Error<architecturekit::DomainError>>
{
    let snapshot = store
        .load::<ProcessInstanceState>(&P::instance_subject(correlation_id))
        .await?;
    let state = if snapshot.started {
        let decoded = serde_json::from_value(snapshot.state.clone()).map_err(|err| {
            architecturekit::Error::Decider(architecturekit::DomainError::new(format!(
                "decoding process manager state for {correlation_id:?}: {err}"
            )))
        })?;
        Some(decoded)
    } else {
        None
    };
    Ok((snapshot, state))
}

#[derive(Clone)]
struct SaveProcessInstance {
    subject: architecturekit::Subject,
    saved: ProcessInstanceSaved,
}

impl architecturekit::Command for SaveProcessInstance {
    fn subject(&self) -> architecturekit::Subject {
        self.subject.clone()
    }

    fn preconditions(&self) -> Vec<architecturekit::Precondition> {
        vec![architecturekit::Precondition::on_state_read()]
    }
}

fn save_instance_decider()
-> architecturekit::Decider<SaveProcessInstance, ProcessInstanceState, architecturekit::DomainError>
{
    architecturekit::Decider::new(
        |cmd: SaveProcessInstance, current: ProcessInstanceState| async move {
            let is_outbox_progress = current.last_event_id == cmd.saved.last_event_id
                && cmd.saved.pending_commands.len() < current.pending_commands.len();
            if !is_outbox_progress
                && ((current.ended && current.pending_commands.is_empty())
                    || current
                        .last_event_id
                        .is_some_and(|last| cmd.saved.last_event_id.is_some_and(|ev| ev <= last)))
            {
                return Ok::<Vec<ProcessInstanceEvent>, architecturekit::DomainError>(Vec::new());
            }
            Ok(vec![ProcessInstanceEvent::Saved(cmd.saved)])
        },
    )
}

async fn save_process_snapshot<P: ProcessManager>(
    store: &architecturekit::Store,
    correlation_id: &str,
    snapshot: &ProcessInstanceState,
) -> Result<(), architecturekit::Error<architecturekit::DomainError>> {
    let saved = ProcessInstanceSaved {
        started: snapshot.started,
        ended: snapshot.ended,
        last_event_id: snapshot.last_event_id,
        active_timers: snapshot.active_timers.clone(),
        pending_commands: snapshot.pending_commands.clone(),
        state: snapshot.state.clone(),
    };
    let decider = save_instance_decider();
    store
        .execute(
            &decider,
            SaveProcessInstance {
                subject: P::instance_subject(correlation_id),
                saved,
            },
        )
        .await?;
    Ok(())
}

/// Persists the updated [`ProcessInstanceSaved`] snapshot (including `pending_commands` outbox)
/// for `correlation_id`, guarded by [`architecturekit::Precondition::on_state_read`] so
/// concurrent updates do not silently overwrite each other.
pub async fn save_process_instance<P: ProcessManager>(
    store: &architecturekit::Store,
    correlation_id: &str,
    event_id: architecturekit::Revision,
    mut active_timers: BTreeMap<String, chrono::DateTime<chrono::Utc>>,
    pending_commands: Vec<serde_json::Value>,
    state: &P,
) -> Result<ProcessInstanceState, architecturekit::Error<architecturekit::DomainError>> {
    let ended = state.is_ended();
    if ended {
        active_timers.clear();
    }
    let state_json = serde_json::to_value(state).map_err(|err| {
        architecturekit::Error::Decider(architecturekit::DomainError::new(format!(
            "encoding process manager state for {correlation_id:?}: {err}"
        )))
    })?;
    let subject = P::instance_subject(correlation_id);
    let saved = ProcessInstanceSaved {
        started: true,
        ended,
        last_event_id: Some(event_id.get()),
        active_timers,
        pending_commands,
        state: state_json,
    };
    let decider = save_instance_decider();
    let written = store
        .execute(
            &decider,
            SaveProcessInstance {
                subject: subject.clone(),
                saved: saved.clone(),
            },
        )
        .await?;
    if written.is_empty() {
        return Ok(store.load::<ProcessInstanceState>(&subject).await?);
    }
    Ok(architecturekit::State::evolve(
        ProcessInstanceState::default(),
        &ProcessInstanceEvent::Saved(saved),
    ))
}

/// Drains and executes any `pending_commands` persisted in `snapshot` for `correlation_id`,
/// checkpointing remaining commands in the outbox if a command fails transiently and clearing
/// `pending_commands` once all queued commands have completed.
pub async fn flush_process_commands<'a, P, F, Fut>(
    store: &'a architecturekit::Store,
    correlation_id: &str,
    snapshot: &mut ProcessInstanceState,
    mut execute_one: F,
) -> Result<(), architecturekit::Error<architecturekit::DomainError>>
where
    P: ProcessManager,
    F: FnMut(serde_json::Value, &'a architecturekit::Store) -> Fut,
    Fut: Future<Output = Result<(), architecturekit::Error<architecturekit::DomainError>>>,
{
    if snapshot.pending_commands.is_empty() {
        return Ok(());
    }
    let initial_len = snapshot.pending_commands.len();
    while let Some(cmd_value) = snapshot.pending_commands.first().cloned() {
        match execute_one(cmd_value, store).await {
            Ok(()) => {
                snapshot.pending_commands.remove(0);
            }
            Err(err) => {
                if snapshot.pending_commands.len() < initial_len {
                    let _ = save_process_snapshot::<P>(store, correlation_id, snapshot).await;
                }
                return Err(err);
            }
        }
    }
    save_process_snapshot::<P>(store, correlation_id, snapshot).await
}

/// Executes an emitted command `C` on consistency unit `S` on behalf of a [`ProcessManager`].
///
/// Business-level command rejections (`Error::Decider`) are logged via [`tracing::warn!`] and
/// isolated (`Ok(())`) so a single rejected command for one process instance does not halt the
/// consumer stream for all other instances, whereas transient conflicts (`Error::Transient`,
/// including `Transient::Conflict`) and store-level infrastructure failures are propagated for retry.
pub async fn execute_process_command<S, C>(
    store: &architecturekit::Store,
    command: C,
) -> Result<(), architecturekit::Error<architecturekit::DomainError>>
where
    S: crate::bounded_context::command::CommandScope,
    C: crate::bounded_context::Command<S>,
{
    crate::integration::event_consumer::execute_isolated_command(store, command, "process manager")
        .await
        .map(|_| ())
}

/// Classifies an action returned from a [`ProcessManager`] reaction into either an
/// emitted domain command, a [`SetTimer`](crate::integration::SetTimer), or a
/// [`CancelTimer`](crate::integration::CancelTimer).
pub trait ProcessCommand: Clone + std::fmt::Debug + PartialEq + Send + Sync + 'static {
    /// If this action arms a timer (`SetTimer<T>`), returns its kebab-case timer name.
    #[must_use]
    fn as_set_timer(&self) -> Option<&'static str> {
        None
    }

    /// If this action cancels a timer (`CancelTimer<T>`), returns its kebab-case timer name.
    #[must_use]
    fn as_cancel_timer(&self) -> Option<&'static str> {
        None
    }

    /// If this action emits a domain command, returns `(bounded_context, target, command, data)`.
    #[must_use]
    fn as_emitted_command(
        &self,
    ) -> Option<(&'static str, &'static str, &'static str, serde_json::Value)> {
        None
    }
}

/// A stateful, long-running coordinator (`kind: process-manager`) that drives a business
/// process across one or more [`BoundedContext`](crate::bounded_context::BoundedContext)s.
///
/// Each instance is correlated by a key drawn from incoming events (`correlated_by = event_field(...)`),
/// starts only on events listed in `starts_when = [...]`, evolves its own state across reactions
/// ([`OnEvent<E>`] and [`OnTimer<T>`]), issues commands from `emits = [...]`, and retires once
/// [`ProcessManager::is_ended`] (`ends_when`) returns `true`.
///
/// # Examples
///
/// ```
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Shipping;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier>, pub canceled: bool }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent {
/// #     OrderPlaced { order_id: Identifier },
/// #     OrderCanceled { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, mut state: Order) -> Order { state.order_id = Some(self.order_id.clone()); state }
/// # }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderCanceled {
/// #     fn evolve(&self, mut state: Order) -> Order { state.canceled = true; state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand {
/// #     #[lifecycle(create)]
/// #     PlaceOrder { order_id: Identifier },
/// #     CancelOrder { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderPlaced { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # static CONFLICT_ONCE: std::sync::Mutex<Option<std::sync::Arc<chromatofoor::architecturekit::Store>>> =
/// #     std::sync::Mutex::new(None);
/// # static SHIPMENT_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
/// # impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         let maybe_store = CONFLICT_ONCE.lock().unwrap().take();
/// #         if let Some(conflict_store) = maybe_store {
/// #             conflict_store
/// #                 .write(
/// #                     vec![chromatofoor::architecturekit::EventOn::new(
/// #                         chromatofoor::architecturekit::Subject::new("/commerce/ordering/order/ord-outbox"),
/// #                         &OrderPlaced {
/// #                             order_id: "ord-outbox".into(),
/// #                         },
/// #                     )],
/// #                     vec![chromatofoor::architecturekit::Precondition::unconditionally()],
/// #                 )
/// #                 .await
/// #                 .unwrap();
/// #         }
/// #         Ok(vec![OrderCanceled { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # #[chromatofoor::aggregate(scope = Billing, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Payment { pub order_id: Option<Identifier>, pub captured: bool }
/// # #[chromatofoor::event(scope = Payment)]
/// # pub enum PaymentEvent { PaymentCaptured { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Payment> for PaymentCaptured {
/// #     fn evolve(&self, mut state: Payment) -> Payment {
/// #         state.order_id = Some(self.order_id.clone());
/// #         state.captured = true;
/// #         state
/// #     }
/// # }
/// # #[chromatofoor::command(scope = Payment)]
/// # pub enum PaymentCommand {
/// #     #[lifecycle(create)]
/// #     CapturePayment { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Payment> for CapturePayment {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Payment) -> Result<Vec<PaymentEvent>, Self::Error> {
/// #         Ok(vec![PaymentCaptured { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # #[chromatofoor::aggregate(scope = Shipping, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Shipment { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Shipment)]
/// # pub enum ShipmentEvent { ShipmentScheduled { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Shipment> for ShipmentScheduled {
/// #     fn evolve(&self, mut state: Shipment) -> Shipment { state.order_id = Some(self.order_id.clone()); state }
/// # }
/// # #[chromatofoor::command(scope = Shipment)]
/// # pub enum ShipmentCommand {
/// #     #[lifecycle(create)]
/// #     ScheduleShipment { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Shipment> for ScheduleShipment {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Shipment) -> Result<Vec<ShipmentEvent>, Self::Error> {
/// #         if self.order_id.as_str() == "ord-rejected" {
/// #             return Err(chromatofoor::architecturekit::DomainError::new("cannot ship"));
/// #         }
/// #         if self.order_id.as_str() == "ord-outbox" {
/// #             SHIPMENT_CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
/// #         }
/// #         Ok(vec![ShipmentScheduled { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # use std::sync::Arc;
/// # use chromatofoor::InMemoryEventStore;
/// # use chromatofoor::architecturekit::{
/// #     CancellationToken, Envelope, EventOn, Precondition, Store, Subject, wait_caught_up,
/// # };
/// # use chromatofoor::bounded_context::Command;
/// # use chromatofoor::common::{Identifier, Kind, Topic};
/// # use chromatofoor::integration::{
/// #     EventConsumer, OnEvent, OnTimer, ProcessInstanceState, ProcessManager, ProcessTimer,
/// #     ProcessTimerFired, TimerSchedule, TimerUnit,
/// # };
/// #[chromatofoor::process_manager(
///     scope = Commerce,
///     correlated_by = event_field(order_id),
///     starts_when = [OrderPlaced],
///     handles = [OrderPlaced, PaymentCaptured],
///     emits = [ScheduleShipment, CancelOrder],
///     timers = [PaymentTimeout(after(minutes = 15))],
///     ends_when = |state: &Self| state.shipped || state.canceled,
///     delivery_guarantee = at_least_once,
///     idempotency = self,
/// )]
/// #[derive(Clone, Default)]
/// pub struct OrderFulfillment {
///     pub order_id: Option<Identifier>,
///     pub shipped: bool,
///     pub canceled: bool,
/// }
///
/// impl OnEvent<OrderPlaced> for OrderFulfillment {
///     fn on_event(&mut self, event: Envelope<OrderPlaced>) -> Vec<Self::Command> {
///         self.order_id = Some(event.data.order_id);
///         vec![PaymentTimeout::set().into()]
///     }
/// }
///
/// impl OnEvent<PaymentCaptured> for OrderFulfillment {
///     fn on_event(&mut self, event: Envelope<PaymentCaptured>) -> Vec<Self::Command> {
///         self.shipped = true;
///         if event.data.order_id.as_str() == "ord-outbox" {
///             return vec![
///                 PaymentTimeout::cancel().into(),
///                 ScheduleShipment {
///                     order_id: event.data.order_id.clone(),
///                 }
///                 .into(),
///                 CancelOrder {
///                     order_id: event.data.order_id,
///                 }
///                 .into(),
///             ];
///         }
///         vec![
///             PaymentTimeout::cancel().into(),
///             ScheduleShipment {
///                 order_id: event.data.order_id,
///             }
///             .into(),
///         ]
///     }
/// }
///
/// impl OnTimer<PaymentTimeout> for OrderFulfillment {
///     fn on_timer(&mut self) -> Vec<Self::Command> {
///         self.canceled = true;
///         vec![
///             CancelOrder {
///                 order_id: self.order_id.clone().unwrap(),
///             }
///             .into(),
///         ]
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// # let token = CancellationToken::new();
/// assert_eq!(OrderFulfillment::KIND, "process-manager");
/// assert_eq!(OrderFulfillment::TOPIC, "/commerce/order-fulfillment");
/// assert_eq!(
///     PaymentTimeout::SCHEDULE,
///     TimerSchedule::After {
///         value: 15,
///         unit: TimerUnit::Minutes,
///     }
/// );
///
/// let run = OrderFulfillment::default().start(Arc::clone(&store), token.clone());
///
/// // 1. Order "ord-1" is placed and paid before timeout -> shipment scheduled, timer cancelled:
/// store
///     .execute(&PlaceOrder::decider(), PlaceOrder { order_id: "ord-1".into() })
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// assert!(
///     OrderFulfillment::active_timers(&store, "ord-1")
///         .await
///         .unwrap()
///         .contains_key("payment-timeout")
/// );
///
/// store
///     .execute(&CapturePayment::decider(), CapturePayment { order_id: "ord-1".into() })
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
///
/// let instance1 = OrderFulfillment::load(&store, "ord-1").await.unwrap().unwrap();
/// assert!(instance1.shipped && instance1.is_ended());
/// assert!(OrderFulfillment::active_timers(&store, "ord-1").await.unwrap().is_empty());
///
/// let shipment: Shipment = store
///     .load(&Subject::new("/commerce/shipping/shipment/ord-1"))
///     .await
///     .unwrap();
/// assert_eq!(shipment.order_id, Some("ord-1".into()));
///
/// // 2. Order "ord-2" is placed; a stale `ProcessTimerFired` with a mismatched deadline is
/// // ignored without removing the active timer, whereas firing with the active timer succeeds:
/// store
///     .execute(&PlaceOrder::decider(), PlaceOrder { order_id: "ord-2".into() })
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
///
/// let armed_deadline = *OrderFulfillment::active_timers(&store, "ord-2")
///     .await
///     .unwrap()
///     .get("payment-timeout")
///     .unwrap();
/// store
///     .write(
///         vec![EventOn::new(
///             Subject::new("/commerce/order-fulfillment/timers/ord-2"),
///             &ProcessTimerFired {
///                 process_manager: OrderFulfillment::TOPIC.to_owned(),
///                 correlation_id: "ord-2".to_owned(),
///                 timer: "payment-timeout".to_owned(),
///                 deadline: Some(armed_deadline - chromatofoor::chrono::Duration::minutes(5)),
///             },
///         )],
///         vec![Precondition::unconditionally()],
///     )
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// assert_eq!(
///     OrderFulfillment::active_timers(&store, "ord-2")
///         .await
///         .unwrap()
///         .get("payment-timeout"),
///     Some(&armed_deadline)
/// );
///
/// OrderFulfillment::fire_timer::<PaymentTimeout>(&store, "ord-2")
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
///
/// let order2: Order = store
///     .load(&Subject::new("/commerce/ordering/order/ord-2"))
///     .await
///     .unwrap();
/// assert!(order2.canceled);
///
/// // 3. Order "ord-rejected" has its downstream `ScheduleShipment` rejected by the target
/// // decider, yet the process manager saves its updated instance state first and isolates the
/// // rejection without halting the stream:
/// store
///     .execute(
///         &PlaceOrder::decider(),
///         PlaceOrder {
///             order_id: "ord-rejected".into(),
///         },
///     )
///     .await
///     .unwrap();
/// store
///     .execute(
///         &CapturePayment::decider(),
///         CapturePayment {
///             order_id: "ord-rejected".into(),
///         },
///     )
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// let rejected_instance = OrderFulfillment::load(&store, "ord-rejected")
///     .await
///     .unwrap()
///     .unwrap();
/// assert!(rejected_instance.shipped && rejected_instance.is_ended());
/// assert!(
///     OrderFulfillment::active_timers(&store, "ord-rejected")
///         .await
///         .unwrap()
///         .is_empty()
/// );
/// token.cancel();
///
/// // 4. Order "ord-outbox" emits two commands where the second (`CancelOrder`) encounters
/// // `Transient::Conflict` after `save_process_instance` has already committed `last_event_id`
/// // and `ended = true`. `AtLeastOnce` retries `handle`, draining only the remaining command
/// // from `pending_commands` without re-executing `ScheduleShipment`:
/// store
///     .execute(
///         &PlaceOrder::decider(),
///         PlaceOrder {
///             order_id: "ord-outbox".into(),
///         },
///     )
///     .await
///     .unwrap();
/// *CONFLICT_ONCE.lock().unwrap() = Some(Arc::clone(&store));
/// store
///     .execute(
///         &CapturePayment::decider(),
///         CapturePayment {
///             order_id: "ord-outbox".into(),
///         },
///     )
///     .await
///     .unwrap();
/// let token2 = CancellationToken::new();
/// let run2 = OrderFulfillment::default().start(Arc::clone(&store), token2.clone());
/// wait_caught_up(&[&run2]).await.unwrap();
/// token2.cancel();
///
/// assert_eq!(SHIPMENT_CALLS.load(std::sync::atomic::Ordering::SeqCst), 1);
/// let final_snapshot: ProcessInstanceState = store
///     .load(&OrderFulfillment::instance_subject("ord-outbox"))
///     .await
///     .unwrap();
/// assert!(final_snapshot.ended);
/// assert!(final_snapshot.pending_commands.is_empty());
/// let outbox_shipment: Shipment = store
///     .load(&Subject::new("/commerce/shipping/shipment/ord-outbox"))
///     .await
///     .unwrap();
/// assert_eq!(outbox_shipment.order_id, Some("ord-outbox".into()));
/// let outbox_order: Order = store
///     .load(&Subject::new("/commerce/ordering/order/ord-outbox"))
///     .await
///     .unwrap();
/// assert!(outbox_order.canceled);
/// # });
/// ```
///
/// Every event in `starts_when` and `handles` must carry the `correlated_by` field and belong
/// to the same [`Domain`](crate::domain::Domain):
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced { pub wrong_field: Identifier }
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderCanceled }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderCanceled {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand { CancelOrder { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderCanceled {}.into()])
/// #     }
/// # }
/// // Error: `OrderPlaced` has no field named `order_id`
/// #[chromatofoor::process_manager(
///     scope = Commerce,
///     correlated_by = event_field(order_id),
///     starts_when = [OrderPlaced],
///     emits = [CancelOrder],
///     ends_when = |_: &Self| true,
///     delivery_guarantee = at_most_once,
/// )]
/// #[derive(Clone, Default)]
/// pub struct BadCorrelationSaga;
/// # impl chromatofoor::integration::OnEvent<OrderPlaced> for BadCorrelationSaga {
/// #     fn on_event(&mut self, _event: chromatofoor::architecturekit::Envelope<OrderPlaced>) -> Vec<Self::Command> {
/// #         vec![]
/// #     }
/// # }
/// ```
pub trait ProcessManager:
    EventConsumer
    + Clone
    + Default
    + serde::Serialize
    + serde::de::DeserializeOwned
    + Send
    + Sync
    + 'static
{
    /// The generated sum type of all commands (and timer actions) emitted by reactions.
    type Command: ProcessCommand;

    /// Returns `true` when the process instance has reached a terminal state (`endsWhen` in ESDM).
    ///
    /// Evaluated after every reaction; once `true`, the instance is retired, any active timers
    /// are cancelled, and subsequent events for the same correlation key are ignored.
    fn is_ended(&self) -> bool;

    /// Applies a single incoming event or timer firing to an in-memory instance during
    /// Given-When-Then testing ([`chromatofoor::testing::given_process_manager`](crate::testing::given_process_manager)).
    #[doc(hidden)]
    fn step(
        state: &mut Option<Self>,
        ended: &mut bool,
        active_timers: &mut std::collections::BTreeSet<&'static str>,
        event: architecturekit::Envelope<Self::Event>,
    ) -> Vec<Self::Command>;

    /// Returns the subject in EventSourcingDB where state for `correlation_id` is persisted.
    #[must_use]
    fn instance_subject(correlation_id: &str) -> architecturekit::Subject {
        architecturekit::Subject::new(&format!(
            "/process-managers{}/{correlation_id}",
            Self::TOPIC
        ))
    }

    /// Loads the current state of the process instance identified by `correlation_id`,
    /// or `None` if no instance has been started for that key.
    fn load(
        store: &architecturekit::Store,
        correlation_id: &str,
    ) -> impl Future<
        Output = Result<Option<Self>, architecturekit::Error<architecturekit::DomainError>>,
    > + Send {
        async move {
            let (_, state) = load_process_instance::<Self>(store, correlation_id).await?;
            Ok(state)
        }
    }

    /// Returns the currently armed timers (`ProcessTimer::NAME` -> target UTC deadline) for `correlation_id`.
    fn active_timers(
        store: &architecturekit::Store,
        correlation_id: &str,
    ) -> impl Future<
        Output = Result<
            BTreeMap<String, chrono::DateTime<chrono::Utc>>,
            architecturekit::LoadError,
        >,
    > + Send {
        async move {
            let snapshot: ProcessInstanceState =
                store.load(&Self::instance_subject(correlation_id)).await?;
            Ok(snapshot.active_timers)
        }
    }

    /// Fires timer `T` for the process instance identified by `correlation_id`.
    fn fire_timer<T: ProcessTimer<Self>>(
        store: &architecturekit::Store,
        correlation_id: &str,
    ) -> impl Future<
        Output = Result<Vec<architecturekit::eventsourcingdb::Event>, architecturekit::WriteError>,
    > + Send {
        let subject =
            architecturekit::Subject::new(&format!("{}/timers/{correlation_id}", Self::TOPIC));
        let event = ProcessTimerFired {
            process_manager: Self::TOPIC.to_owned(),
            correlation_id: correlation_id.to_owned(),
            timer: T::NAME.to_owned(),
            deadline: None,
        };
        async move {
            store
                .write(
                    vec![architecturekit::EventOn::new(subject, &event)],
                    vec![architecturekit::Precondition::unconditionally()],
                )
                .await
        }
    }
}

/// Per-event reaction logic (`reactions[].when` with `eventReference`) for a [`ProcessManager`].
pub trait OnEvent<E>: ProcessManager {
    /// Mutates the process instance state in response to `event` and returns commands
    /// (or timer actions) to execute.
    fn on_event(&mut self, event: architecturekit::Envelope<E>) -> Vec<Self::Command>;
}

/// Per-timer reaction logic (`reactions[].when` with `{ timer: <name> }`) for a [`ProcessManager`].
pub trait OnTimer<T: ProcessTimer<Self>>: ProcessManager {
    /// Mutates the process instance state when timer `T` fires and returns commands
    /// (or timer actions) to execute.
    fn on_timer(&mut self) -> Vec<Self::Command>;
}
