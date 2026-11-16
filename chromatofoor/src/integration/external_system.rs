//! Third-party or out-of-scope services (`kind: external-system`) scoped to a [`Domain`].

use crate::{common::Kind, domain::Domain};

/// Direction of interaction with an [`ExternalSystem`] (`external-system.direction` in ESDM).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalDirection {
    /// The external system calls into the domain (e.g. webhooks, push notifications).
    Inbound,
    /// The domain calls out to the external system (e.g. sending emails, charging cards).
    Outbound,
    /// Both inbound and outbound interactions are in use.
    Bidirectional,
}

/// A third-party or out-of-scope dependency (`kind: external-system`) scoped to a [`Domain`].
///
/// External systems live at domain scope so that a single provider (e.g. `Stripe`, `SendGrid`)
/// can be referenced across bounded contexts without duplication, while [`Self::DIRECTION`]
/// enforces how the domain interacts with it at compile time:
/// - **Outbound (`outbound` | `bidirectional`)**: an [`EventHandler`](crate::integration::EventHandler)
///   declares `external_systems = [Stripe]` and invokes the external system as a side effect when
///   handling domain events.
/// - **Inbound (`inbound` | `bidirectional`)**: a `type = system` [`Actor`](crate::bounded_context::Actor)
///   declares `backed_by = [Stripe]` and is authorized on internal [`Command`](crate::bounded_context::Command)s
///   triggered when the external system calls back into the domain (e.g. via a webhook).
///
/// # Examples
///
/// End-to-end payment flow with `Stripe` (`direction = bidirectional`):
/// 1. **Outbound**: When `OrderPlaced` occurs, `StripeCheckoutHandler` (`external_systems = [Stripe]`)
///    calls `self.stripe.create_charge(...)`.
/// 2. **Inbound**: When `Stripe` sends a webhook callback, it acts as the `PaymentWebhook` system
///    actor (`backed_by = [Stripe]`) to execute the internal-only `RecordPayment` command.
///
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use chromatofoor::{
/// #     architecturekit::{
/// #         CancellationToken, DomainError, Envelope, InMemoryEventStore, Store, Subject,
/// #         wait_caught_up,
/// #     },
/// #     bounded_context::{Actor, Authorized, Command},
/// #     common::{Identifier, Kind, Topic},
/// #     integration::{
/// #         EventConsumer, EventHandler, ExternalDirection, ExternalSystem, Handle,
/// #     },
/// # };
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Ordering;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Billing;
///
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced { order_id: Identifier, amount_cents: u64 } }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, mut state: Order) -> Order { state.order_id = Some(self.order_id.clone()); state }
/// # }
/// # #[chromatofoor::command(scope = Order)]
/// # pub enum OrderCommand {
/// #     #[lifecycle(create)]
/// #     PlaceOrder { order_id: Identifier, amount_cents: u64 },
/// # }
/// # impl Command<Order> for PlaceOrder {
/// #     type Error = DomainError;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderPlaced { order_id: self.order_id, amount_cents: self.amount_cents }.into()])
/// #     }
/// # }
/// // 1. Declare `Stripe` as a bidirectional external system in the `Commerce` domain:
/// #[chromatofoor::external_system(
///     scope = Commerce,
///     direction = bidirectional,
///     category = "payment",
/// )]
/// #[derive(Clone, Default)]
/// pub struct Stripe {
///     pub charges: Arc<Mutex<Vec<(String, u64)>>>,
/// }
///
/// impl Stripe {
///     pub fn create_charge(&self, order_id: &str, amount_cents: u64) {
///         self.charges.lock().unwrap().push((order_id.to_owned(), amount_cents));
///     }
/// }
///
/// // 2. Outbound: `StripeCheckoutHandler` reacts to `OrderPlaced` by calling `Stripe`:
/// #[chromatofoor::event_handler(
///     scope = Commerce,
///     handles = [OrderPlaced],
///     external_systems = [Stripe],
///     delivery_guarantee = at_least_once,
///     idempotency(owner = downstream, strategy = "Stripe idempotency key per order_id"),
/// )]
/// #[derive(Clone, Default)]
/// pub struct StripeCheckoutHandler {
///     pub stripe: Stripe,
/// }
///
/// impl Handle<OrderPlaced> for StripeCheckoutHandler {
///     async fn handle(&self, event: Envelope<OrderPlaced>) -> Result<(), DomainError> {
///         self.stripe.create_charge(event.data.order_id.as_str(), event.data.amount_cents);
///         Ok(())
///     }
/// }
///
/// // 3. Inbound: `PaymentWebhook` is an internal `system` actor backed by `Stripe`:
/// #[chromatofoor::actor(scope = Billing, type = system, backed_by = [Stripe])]
/// pub struct PaymentWebhook;
///
/// #[chromatofoor::aggregate(scope = Billing, identified_by = state(order_id))]
/// #[derive(Clone, Default)]
/// pub struct Payment {
///     pub order_id: Option<Identifier>,
///     pub settled: bool,
/// }
/// # #[chromatofoor::event(scope = Payment)]
/// # pub enum PaymentEvent { PaymentSettled { order_id: Identifier } }
/// # impl chromatofoor::bounded_context::Event<Payment> for PaymentSettled {
/// #     fn evolve(&self, mut state: Payment) -> Payment {
/// #         state.order_id = Some(self.order_id.clone());
/// #         state.settled = true;
/// #         state
/// #     }
/// # }
///
/// #[chromatofoor::command(scope = Payment, actors = [PaymentWebhook])]
/// pub enum PaymentCommand {
///     #[lifecycle(create)]
///     RecordPayment { order_id: Identifier },
/// }
/// # impl Command<Payment> for RecordPayment {
/// #     type Error = DomainError;
/// #     async fn decide(self, _state: &Payment) -> Result<Vec<PaymentEvent>, Self::Error> {
/// #         Ok(vec![PaymentSettled { order_id: self.order_id }.into()])
/// #     }
/// # }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// # let store = Arc::new(Store::builder(InMemoryEventStore::new(), "https://commerce.test").build());
/// # let token = CancellationToken::new();
/// let stripe = Stripe::default();
/// let handler = StripeCheckoutHandler { stripe: stripe.clone() };
/// let run = handler.start(Arc::clone(&store), token.clone());
///
/// // Customer places an order -> `StripeCheckoutHandler` calls `stripe.create_charge(...)`:
/// store
///     .execute(&PlaceOrder::decider(), PlaceOrder { order_id: "ord-1".into(), amount_cents: 4900 })
///     .await
///     .unwrap();
/// wait_caught_up(&[&run]).await.unwrap();
/// assert_eq!(*stripe.charges.lock().unwrap(), vec![("ord-1".to_owned(), 4900)]);
///
/// // Stripe webhook calls back as `PaymentWebhook` to settle `Payment`:
/// assert!(RecordPayment::allows::<PaymentWebhook>());
/// store
///     .execute(&RecordPayment::decider(), RecordPayment { order_id: "ord-1".into() })
///     .await
///     .unwrap();
/// # token.cancel();
///
/// let payment: Payment = store.load(&Subject::new("/commerce/billing/payment/ord-1")).await.unwrap();
/// assert!(payment.settled);
/// assert_eq!(Stripe::DIRECTION, ExternalDirection::Bidirectional);
/// assert_eq!(PaymentWebhook::BACKED_BY, &["stripe"]);
/// assert_eq!(StripeCheckoutHandler::EXTERNAL_SYSTEMS, &["stripe"]);
/// # });
/// ```
///
/// ## Compile-time checks
///
/// 1. Specifying `backed_by` on a `type = human` [`Actor`](crate::bounded_context::Actor) is rejected:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// # #[chromatofoor::external_system(scope = Commerce, direction = inbound)]
/// # pub struct Stripe;
/// // Error: `backed_by` is only allowed on `type = system` actors
/// #[chromatofoor::actor(scope = Billing, type = human, backed_by = [Stripe])]
/// pub struct HumanWithWebhook;
/// ```
///
/// 2. Using an `outbound`-only [`ExternalSystem`] in `backed_by` (or an `inbound`-only
///    system in `external_systems` on `#[event_handler]`) is rejected:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// #[chromatofoor::external_system(scope = Commerce, direction = outbound, category = "mail")]
/// pub struct SendGrid;
///
/// // Error: `SendGrid` is `outbound`-only and cannot back an inbound system actor
/// #[chromatofoor::actor(scope = Billing, type = system, backed_by = [SendGrid])]
/// pub struct InboundMailActor;
/// ```
///
/// 3. Referencing an [`ExternalSystem`] from a different [`Domain`] is rejected:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::domain]
/// # pub struct Logistics;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// #[chromatofoor::external_system(scope = Logistics, direction = inbound)]
/// pub struct CarrierWebhookSource;
///
/// // Error: `CarrierWebhookSource` belongs to `Logistics`, not `Commerce`
/// #[chromatofoor::actor(scope = Billing, type = system, backed_by = [CarrierWebhookSource])]
/// pub struct WrongDomainActor;
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `ExternalSystem`",
    label = "expected an `#[external_system]` type here"
)]
pub trait ExternalSystem: Kind<Scope: Domain> {
    /// Whether this external system is `inbound`, `outbound`, or `bidirectional`.
    const DIRECTION: ExternalDirection;

    /// Optional short tag describing the category of external system (e.g. `"payment"`, `"mail"`).
    const CATEGORY: Option<&'static str> = None;
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "external system `{Self}` does not belong to domain `{D}`",
    label = "external system `{Self}` belongs to a different domain than `{D}`",
    note = "`backed_by` and `external_systems` can only reference external systems scoped to the same domain"
)]
pub trait ExternalSystemInDomain<D: Domain>: ExternalSystem {}

#[doc(hidden)]
pub const fn assert_external_system_in_domain<D: Domain, E: ExternalSystemInDomain<D>>() {}
