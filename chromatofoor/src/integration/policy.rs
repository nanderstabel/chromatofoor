//! Stateless event-to-command reactions scoped to a [`Domain`](crate::domain::Domain).

use crate::integration::event_consumer::EventConsumer;

/// A stateless, automated reaction scoped to a [`Domain`](crate::domain::Domain) that consumes
/// domain events and emits commands to drive subsequent state transitions across one or
/// more [`BoundedContext`](crate::bounded_context::BoundedContext)s.
///
/// Derived via [`#[chromatofoor::policy(...)]`](chromatofoor_derive::policy),
/// with per-event reaction logic implemented via [`React<E>`].
///
/// # Examples
///
/// ```rust
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Shipping;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, mut state: Order) -> Order {
/// #         state.order_id = Some(self.order_id.clone());
/// #         state
/// #     }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand {
/// #     #[lifecycle(create)]
/// #     PlaceOrder { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderPlaced { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # #[chromatofoor::aggregate(scope = Shipping, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Shipment { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Shipment)]
/// # pub enum ShipmentEvent { ShipmentScheduled { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Shipment> for ShipmentScheduled {
/// #     fn evolve(&self, mut state: Shipment) -> Shipment {
/// #         state.order_id = Some(self.order_id.clone());
/// #         state
/// #     }
/// # }
/// # #[chromatofoor::command(scope = Shipment)]
/// # pub enum ShipmentCommand {
/// #     #[lifecycle(create)]
/// #     ScheduleShipment { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Shipment> for ScheduleShipment {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Shipment) -> Result<Vec<ShipmentEvent>, Self::Error> {
/// #         if self.order_id.as_str() == "ord-blocked" {
/// #             return Err(chromatofoor::architecturekit::DomainError::new("shipment blocked"));
/// #         }
/// #         Ok(vec![ShipmentScheduled { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # use std::sync::Arc;
/// # use chromatofoor::{
/// #     InMemoryEventStore,
/// #     architecturekit::{CancellationToken, Envelope, Store, Subject, wait_caught_up},
/// #     bounded_context::Command,
/// #     common::Identifier,
/// #     integration::{
/// #         DeliveryGuarantee, EventConsumer, IdempotencyOwner, Policy, React,
/// #         execute_policy_command,
/// #     },
/// # };
/// #[chromatofoor::policy(
///     scope = Commerce,
///     handles = [OrderPlaced],
///     emits = [ScheduleShipment],
///     delivery_guarantee = at_least_once,
///     idempotency = downstream,
/// )]
/// pub struct FulfillmentPolicy;
///
/// impl React<OrderPlaced> for FulfillmentPolicy {
///     fn react(&self, event: Envelope<OrderPlaced>) -> Vec<Self::Command> {
///         vec![
///             ScheduleShipment {
///                 order_id: event.data.order_id,
///             }
///             .into(),
///         ]
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// # let token = CancellationToken::new();
/// store
///     .execute(
///         &PlaceOrder::decider(),
///         PlaceOrder {
///             order_id: "ord-1".into(),
///         },
///     )
///     .await
///     .unwrap();
///
/// let run = FulfillmentPolicy.start(Arc::clone(&store), token.clone());
/// wait_caught_up(&[&run]).await.unwrap();
///
/// // A rejected downstream command ("ord-blocked") is isolated and does not halt the stream
/// // for subsequent events ("ord-2"):
/// store
///     .execute(
///         &PlaceOrder::decider(),
///         PlaceOrder {
///             order_id: "ord-blocked".into(),
///         },
///     )
///     .await
///     .unwrap();
/// store
///     .execute(
///         &PlaceOrder::decider(),
///         PlaceOrder {
///             order_id: "ord-2".into(),
///         },
///     )
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// # token.cancel();
///
/// let shipment: Shipment = store
///     .load(&Subject::new("/commerce/shipping/shipment/ord-1"))
///     .await
///     .unwrap();
/// assert_eq!(shipment.order_id, Some("ord-1".into()));
/// let shipment2: Shipment = store
///     .load(&Subject::new("/commerce/shipping/shipment/ord-2"))
///     .await
///     .unwrap();
/// assert_eq!(shipment2.order_id, Some("ord-2".into()));
/// assert!(
///     execute_policy_command::<Shipment, _>(
///         &store,
///         ScheduleShipment {
///             order_id: "ord-blocked".into(),
///         },
///     )
///     .await
///     .unwrap()
///     .is_empty()
/// );
/// assert_eq!(
///     FulfillmentPolicy::DELIVERY_GUARANTEE,
///     DeliveryGuarantee::AtLeastOnce
/// );
/// assert_eq!(
///     FulfillmentPolicy::IDEMPOTENCY.unwrap().owner,
///     IdempotencyOwner::Downstream
/// );
/// # });
/// ```
///
/// ## Compile-time checks
///
/// Specifying `delivery_guarantee = at_least_once` without `idempotency` is rejected:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced { pub order_id: Identifier }
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderConfirmed }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderConfirmed {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand { ConfirmOrder { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Command<Order> for ConfirmOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderConfirmed {}.into()])
/// #     }
/// # }
/// // Error: `delivery_guarantee = at_least_once` requires `idempotency = ...`
/// #[chromatofoor::policy(
///     scope = Commerce,
///     handles = [OrderPlaced],
///     emits = [ConfirmOrder],
///     delivery_guarantee = at_least_once,
/// )]
/// pub struct InvalidPolicy;
/// # impl chromatofoor::integration::React<OrderPlaced> for InvalidPolicy {
/// #     fn react(&self, event: chromatofoor::architecturekit::Envelope<OrderPlaced>) -> Vec<Self::Command> {
/// #         vec![ConfirmOrder { order_id: event.data.order_id }.into()]
/// #     }
/// # }
/// ```
///
/// Handling events or emitting commands from a different [`Domain`](crate::domain::Domain) is also rejected:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::domain]
/// # pub struct Logistics;
/// # #[chromatofoor::bounded_context(scope = Logistics)]
/// # pub struct Warehouse;
/// # #[chromatofoor::event(scope = Warehouse)]
/// # pub struct StockReceived;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderConfirmed }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderConfirmed {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand { ConfirmOrder { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Command<Order> for ConfirmOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderConfirmed {}.into()])
/// #     }
/// # }
/// // Error: `StockReceived` belongs to `Logistics`, not `Commerce`
/// #[chromatofoor::policy(
///     scope = Commerce,
///     handles = [StockReceived],
///     emits = [ConfirmOrder],
///     delivery_guarantee = at_most_once,
/// )]
/// pub struct CrossDomainPolicy;
/// # impl chromatofoor::integration::React<StockReceived> for CrossDomainPolicy {
/// #     fn react(&self, _event: chromatofoor::architecturekit::Envelope<StockReceived>) -> Vec<Self::Command> {
/// #         vec![]
/// #     }
/// # }
/// ```
pub trait Policy: EventConsumer {
    /// The generated sum type of all commands emitted by this policy.
    type Command;

    /// Dispatches an incoming event envelope to the corresponding [`React<E>`] implementation.
    fn react(&self, event: architecturekit::Envelope<Self::Event>) -> Vec<Self::Command>;
}

/// Per-event reaction logic for a [`Policy`].
pub trait React<E>: Policy {
    /// Computes zero or more commands to emit in response to `event`.
    fn react(&self, event: architecturekit::Envelope<E>) -> Vec<Self::Command>;
}

/// Executes an emitted command `C` on consistency unit `S` on behalf of a [`Policy`].
///
/// Business-level command rejections (`Error::Decider`) are logged via [`tracing::warn!`] and
/// isolated (`Ok(Vec::new())`) so a single rejected command does not permanently halt an
/// `AtLeastOnce` policy projection stream for all subsequent events, whereas transient
/// conflicts (`Error::Transient`, including `Transient::Conflict`) and store-level
/// infrastructure failures are propagated for retry.
pub async fn execute_policy_command<S, C>(
    store: &architecturekit::Store,
    command: C,
) -> Result<
    Vec<architecturekit::eventsourcingdb::Event>,
    architecturekit::Error<architecturekit::DomainError>,
>
where
    S: crate::bounded_context::command::CommandScope,
    C: crate::bounded_context::Command<S>,
{
    crate::integration::event_consumer::execute_isolated_command(store, command, "policy").await
}
