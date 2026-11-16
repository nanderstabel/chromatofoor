use std::future::Future;

use crate::integration::EventConsumer;

/// A stateless, domain-scoped reaction (`kind: event-handler`) that performs externally
/// observable side effects (sending emails, calling third-party APIs, writing audit logs)
/// when events in `handles` occur, without mutating domain state.
///
/// Use [`#[chromatofoor::event_handler]`](chromatofoor_derive::event_handler) to generate the
/// `<Name>Events` sum type, [`EventConsumer`] implementation, and compile-time checks that
/// all handled events belong to the same [`Domain`](crate::domain::Domain). Implement
/// [`Handle<E>`] for every event listed in `handles = [...]`.
///
/// # Examples
///
/// ```
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, _state: Order) -> Order { Order { order_id: Some(self.order_id.clone()) } }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand {
/// #     #[lifecycle(create)]
/// #     PlaceOrder { order_id: Identifier },
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = chromatofoor::architecturekit::DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderPlaced { order_id: self.order_id }.into()])
/// #     }
/// # }
/// # use std::sync::{Arc, Mutex};
/// # use chromatofoor::architecturekit::{
/// #     CancellationToken, DomainError, Envelope, InMemoryEventStore, Store, wait_caught_up,
/// # };
/// # use chromatofoor::bounded_context::Command;
/// # use chromatofoor::common::{Identifier, Kind, Topic};
/// # use chromatofoor::integration::{
/// #     DeliveryGuarantee, EventConsumer, Handle, Idempotency, IdempotencyOwner,
/// # };
/// #[chromatofoor::event_handler(
///     scope = Commerce,
///     handles = [OrderPlaced],
///     delivery_guarantee = at_least_once,
///     idempotency(owner = downstream, strategy = "SES MessageId deduplication"),
/// )]
/// #[derive(Clone, Default)]
/// pub struct OrderConfirmationMailer {
///     pub sent: Arc<Mutex<Vec<String>>>,
/// }
///
/// impl Handle<OrderPlaced> for OrderConfirmationMailer {
///     async fn handle(&self, event: Envelope<OrderPlaced>) -> Result<(), DomainError> {
///         self.sent
///             .lock()
///             .unwrap()
///             .push(format!("confirmed:{}", event.data.order_id));
///         Ok(())
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
/// let mailer = OrderConfirmationMailer::default();
/// let run = mailer.clone().start(Arc::clone(&store), token.clone());
/// wait_caught_up(&[&run]).await.unwrap();
/// # token.cancel();
///
/// assert_eq!(*mailer.sent.lock().unwrap(), vec!["confirmed:ord-1"]);
/// assert_eq!(OrderConfirmationMailer::KIND, "event-handler");
/// assert_eq!(
///     OrderConfirmationMailer::TOPIC,
///     "/commerce/order-confirmation-mailer"
/// );
/// assert_eq!(
///     OrderConfirmationMailer::DELIVERY_GUARANTEE,
///     DeliveryGuarantee::AtLeastOnce
/// );
/// assert_eq!(
///     OrderConfirmationMailer::IDEMPOTENCY,
///     Some(Idempotency {
///         owner: IdempotencyOwner::Downstream,
///         strategy: Some("SES MessageId deduplication"),
///     })
/// );
/// # });
/// ```
///
/// Both `handles` events from a different [`Domain`](crate::domain::Domain) and
/// `delivery_guarantee = at_least_once` without `idempotency` are rejected at compile time:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::domain]
/// # pub struct Logistics;
/// # #[chromatofoor::bounded_context(scope = Logistics)]
/// # pub struct Warehouse;
/// # #[chromatofoor::event(scope = Warehouse)]
/// # pub struct StockReceived;
/// // Error: `StockReceived` belongs to `Logistics`, not `Commerce`
/// #[chromatofoor::event_handler(
///     scope = Commerce,
///     handles = [StockReceived],
///     delivery_guarantee = at_most_once,
/// )]
/// pub struct CrossDomainHandler;
/// # impl chromatofoor::integration::Handle<StockReceived> for CrossDomainHandler {
/// #     async fn handle(&self, _event: chromatofoor::architecturekit::Envelope<StockReceived>) -> Result<(), chromatofoor::architecturekit::DomainError> {
/// #         Ok(())
/// #     }
/// # }
/// ```
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced;
/// // Error: `delivery_guarantee = at_least_once` requires `idempotency = ...`
/// #[chromatofoor::event_handler(
///     scope = Commerce,
///     handles = [OrderPlaced],
///     delivery_guarantee = at_least_once,
/// )]
/// pub struct MissingIdempotencyHandler;
/// # impl chromatofoor::integration::Handle<OrderPlaced> for MissingIdempotencyHandler {
/// #     async fn handle(&self, _event: chromatofoor::architecturekit::Envelope<OrderPlaced>) -> Result<(), chromatofoor::architecturekit::DomainError> {
/// #         Ok(())
/// #     }
/// # }
/// ```
pub trait EventHandler: EventConsumer {
    /// Kebab-case names of [`ExternalSystem`](crate::integration::ExternalSystem)s called as side
    /// effects by this handler (`sideEffects[].externalSystem` in ESDM).
    const EXTERNAL_SYSTEMS: &'static [&'static str] = &[];
}

/// Per-event side-effect reaction for an [`EventHandler`].
pub trait Handle<E>: EventHandler {
    /// Performs the side effect triggered by `event`.
    fn handle(
        &self,
        event: architecturekit::Envelope<E>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
