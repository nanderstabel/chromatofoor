use std::future::Future;

use crate::{
    bounded_context::{
        BoundedContext, Command,
        command::{CommandExecutionError, CommandScope, verify_zero_event_preconditions},
    },
    common::{Identified, Kind},
};

#[doc(hidden)]
pub trait DcbEvents: architecturekit::Events {
    fn event_on(&self, subject: architecturekit::Subject) -> architecturekit::EventOn;
}

/// An ESDM `dynamic-consistency-boundary` (DCB) — a consistency unit defined dynamically
/// per decision across consulted events within a [`BoundedContext`], rather than
/// statically by a single aggregate container.
///
/// # Example
///
/// `CouponRedemption` consults `OrderPlaced` (from the `Order` aggregate) and the
/// freestanding `CouponRedeemed` event to ensure a customer has placed an order and has not
/// already redeemed the coupon:
///
/// ```
/// # use chromatofoor::{
/// #     architecturekit::testing,
/// #     bounded_context::{Command, Event},
/// #     common::Identifier,
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced { order_id: Identifier, customer_id: Identifier } }
/// # impl Event<Order> for OrderPlaced {
/// #     fn evolve(&self, _state: Order) -> Order { Order { order_id: Some(self.order_id.clone()) } }
/// # }
/// #[chromatofoor::event(scope = Ordering)]
/// pub struct CouponRedeemed {
///     pub customer_id: Identifier,
///     pub coupon_code: Identifier,
/// }
///
/// #[chromatofoor::dynamic_consistency_boundary(
///     scope = Ordering,
///     identified_by = [command_payload(customer_id), command_payload(coupon_code)],
///     consults = [OrderPlaced, CouponRedeemed],
/// )]
/// #[derive(Clone, Default)]
/// pub struct CouponRedemption {
///     pub customers_with_orders: Vec<Identifier>,
///     pub redeemed: Vec<(Identifier, Identifier)>,
/// }
///
/// impl Event<CouponRedemption> for OrderPlaced {
///     fn evolve(&self, mut state: CouponRedemption) -> CouponRedemption {
///         state.customers_with_orders.push(self.customer_id.clone());
///         state
///     }
/// }
///
/// impl Event<CouponRedemption> for CouponRedeemed {
///     fn evolve(&self, mut state: CouponRedemption) -> CouponRedemption {
///         state.redeemed.push((self.customer_id.clone(), self.coupon_code.clone()));
///         state
///     }
/// }
///
/// #[chromatofoor::command(scope = CouponRedemption)]
/// pub enum CouponRedemptionCommand {
///     RedeemCoupon { customer_id: Identifier, coupon_code: Identifier },
/// }
///
/// impl Command<CouponRedemption> for RedeemCoupon {
///     type Error = &'static str;
///
///     async fn decide(
///         self,
///         state: &CouponRedemption,
///     ) -> Result<Vec<CouponRedemptionEvent>, Self::Error> {
///         if !state.customers_with_orders.contains(&self.customer_id) {
///             return Err("customer has no orders");
///         }
///         if state.redeemed.contains(&(self.customer_id.clone(), self.coupon_code.clone())) {
///             return Err("coupon already redeemed");
///         }
///         Ok(vec![CouponRedeemed {
///             customer_id: self.customer_id,
///             coupon_code: self.coupon_code,
///         }.into()])
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// testing::given(
///     &CouponRedemptionCommand::decider(),
///     [OrderPlaced { order_id: "ord-1".into(), customer_id: "cust-1".into() }.into()],
/// )
/// .when(RedeemCoupon { customer_id: "cust-1".into(), coupon_code: "WELCOME10".into() }.into())
/// .await
/// .then_events([CouponRedeemed {
///     customer_id: "cust-1".into(),
///     coupon_code: "WELCOME10".into(),
/// }.into()]);
/// # });
/// ```
///
/// Consulting an event from a different [`BoundedContext`] is rejected at compile time:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Shipping;
/// #[chromatofoor::event(scope = Shipping)]
/// pub struct ShipmentDispatched {
///     pub order_id: String,
/// }
///
/// // Error: `ShipmentDispatched` belongs to `Shipping`, not `Ordering`
/// #[chromatofoor::dynamic_consistency_boundary(
///     scope = Ordering,
///     identified_by = [command_payload(order_id)],
///     consults = [ShipmentDispatched],
/// )]
/// #[derive(Clone, Default)]
/// pub struct InvalidBoundary;
/// # impl chromatofoor::bounded_context::Event<InvalidBoundary> for ShipmentDispatched {
/// #     fn evolve(&self, state: InvalidBoundary) -> InvalidBoundary { state }
/// # }
/// ```
///
/// And `consults = [...]` must name at least one event:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::dynamic_consistency_boundary(scope = Ordering, consults = [])]
/// #[derive(Clone, Default)]
/// pub struct EmptyBoundary;
/// ```
pub trait DynamicConsistencyBoundary:
    Kind<Scope: BoundedContext> + Identified + CommandScope + architecturekit::State<Event: DcbEvents>
{
    /// Executes `command` against `store` by consulting all matching events across this
    /// boundary's [`BoundedContext`] subject tree, folding them into `Self`, running
    /// [`Command::decide`], and atomically writing the emitted events to `command.subject()`
    /// guarded by a recursive optimistic-concurrency precondition on the bounded context.
    fn execute<C>(
        store: &architecturekit::Store,
        command: C,
    ) -> impl Future<
        Output = Result<
            Vec<architecturekit::eventsourcingdb::Event>,
            CommandExecutionError<C::Error>,
        >,
    > + Send
    where
        Self: Sync,
        Self::Event: Send + Sync,
        C: Command<Self> + Send + Sync,
    {
        async move {
            use architecturekit::{
                Precondition, Projected, ReadOptions, Subject, Subjects, check_preconditions,
                eventsourcingdb, revision_of,
            };
            use futures_util::StreamExt as _;

            check_preconditions(&command)?;
            let declared_preconditions = architecturekit::Command::preconditions(&command);

            let bc_subject = Subject::new(<Self::Scope as Kind>::TOPIC);
            let subjects = Subjects::tree(bc_subject.clone());
            let options = ReadOptions::default();
            let stream = store.read(&subjects, &options).await?;
            let mut stream = std::pin::pin!(stream);

            let mut state = Self::default();
            let mut last_event_id = None;

            while let Some(item) = stream.next().await {
                let stored = item?;
                last_event_id = revision_of(std::slice::from_ref(&stored));

                let envelopes = <Self::Event as Projected>::project(&stored, Self::upcasters())
                    .map_err(architecturekit::Error::Permanent)?;
                for envelope in envelopes {
                    if command.matches(&envelope) {
                        state = state.evolve(&envelope.data);
                    }
                }
            }

            let subject = command.subject();
            command
                .validate_payload()
                .map_err(architecturekit::Error::Decider)?;
            let events = command
                .decide(&state)
                .await
                .map_err(architecturekit::Error::Decider)?;
            if events.is_empty() {
                verify_zero_event_preconditions(store, &declared_preconditions).await?;
                return Ok(Vec::new());
            }

            let candidates = events
                .iter()
                .map(|ev| ev.event_on(subject.clone()))
                .collect();

            let recursive = Some(eventsourcingdb::PreconditionOptions { recursive: true });
            let precondition = match last_event_id {
                Some(rev) => {
                    Precondition::require(eventsourcingdb::Precondition::is_subject_on_event_id(
                        bc_subject.as_str(),
                        rev.to_string(),
                        recursive,
                    ))
                }
                None => Precondition::require(eventsourcingdb::Precondition::is_subject_pristine(
                    bc_subject.as_str(),
                    recursive,
                )),
            };

            Ok(store.write(candidates, vec![precondition]).await?)
        }
    }
}
