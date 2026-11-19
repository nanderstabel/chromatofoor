use crate::{
    bounded_context::{Aggregate, BoundedContext},
    common::Kind,
};

/// An ESDM `event` — an immutable fact recorded in the event stream that evolves a state `S`
/// ([`Aggregate`] or [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary)).
///
/// ## 1. Aggregate-Scoped Events (`#[event(scope = <Aggregate>)] pub enum ...`)
///
/// Generates a struct for each variant and implements [`architecturekit::State`] for the
/// aggregate by delegating to [`Event::evolve`]:
///
/// ```
/// # use chromatofoor::{architecturekit::{Event as _, State}, bounded_context::Event, common::Identifier};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// #[derive(Clone, Default, Debug, PartialEq)]
/// pub struct Order {
///     pub order_id: Option<Identifier>,
///     pub is_canceled: bool,
/// }
///
/// #[chromatofoor::event(scope = Order)]
/// pub enum OrderEvent {
///     OrderPlaced { order_id: Identifier },
///     OrderCanceled,
/// }
///
/// impl Event<Order> for OrderPlaced {
///     fn evolve(&self, state: Order) -> Order {
///         Order { order_id: Some(self.order_id.clone()), ..state }
///     }
/// }
///
/// impl Event<Order> for OrderCanceled {
///     fn evolve(&self, state: Order) -> Order {
///         Order { is_canceled: true, ..state }
///     }
/// }
///
/// assert_eq!(OrderPlaced::EVENT_TYPE, "/commerce/ordering/order/order-placed");
///
/// let order = Order::default()
///     .evolve(&OrderPlaced { order_id: "ord-1".into() }.into())
///     .evolve(&OrderCanceled {}.into());
/// assert!(order.is_canceled);
/// ```
///
/// ## 2. Freestanding Bounded-Context Events (`#[event(scope = <BoundedContext>)] pub struct ...`)
///
/// Events produced within a [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary)
/// are scoped directly to the [`BoundedContext`] so multiple DCBs can consult and evolve them:
///
/// ```
/// # use chromatofoor::{architecturekit::Event as _, bounded_context::Event};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::event(scope = Ordering)]
/// pub struct CouponRedeemed {
///     pub customer_id: String,
///     pub coupon_code: String,
/// }
/// # #[chromatofoor::dynamic_consistency_boundary(
/// #     scope = Ordering,
/// #     identified_by = [command_payload(customer_id)],
/// #     consults = [CouponRedeemed],
/// # )]
/// # #[derive(Clone, Default)]
/// # pub struct CustomerCouponUsage { pub redeemed_codes: Vec<String> }
///
/// impl Event<CustomerCouponUsage> for CouponRedeemed {
///     fn evolve(&self, mut state: CustomerCouponUsage) -> CustomerCouponUsage {
///         state.redeemed_codes.push(self.coupon_code.clone());
///         state
///     }
/// }
///
/// assert_eq!(CouponRedeemed::EVENT_TYPE, "/commerce/ordering/coupon-redeemed");
/// ```
///
/// ## Compile-Time Guarantees
///
/// Event `enum`s require an [`Aggregate`] scope, while standalone event `struct`s require
/// an [`Aggregate`] or [`BoundedContext`] scope:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering)]
/// # #[derive(Clone, Default)]
/// # pub struct Order;
/// // Error: `Ordering` is a `BoundedContext`, not an `Aggregate`
/// #[chromatofoor::event(scope = Ordering)]
/// pub enum OrderingEvent {
///     CouponRedeemed,
/// }
/// # impl<S: chromatofoor::architecturekit::State> chromatofoor::bounded_context::Event<S> for CouponRedeemed {
/// #     fn evolve(&self, state: S) -> S { state }
/// # }
/// ```
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// // Error: `Commerce` is a `Domain`, not an `Aggregate` or `BoundedContext`
/// #[chromatofoor::event(scope = Commerce)]
/// pub struct OrderPlaced;
/// ```
///
/// And an aggregate event `enum` must declare at least one variant:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering)]
/// # #[derive(Clone, Default)]
/// # pub struct Order;
/// #[chromatofoor::event(scope = Order)]
/// pub enum OrderEvent {}
/// ```
pub trait Event<S: architecturekit::State>: Kind + architecturekit::Event {
    /// Applies this event to `state` and returns the updated state.
    fn evolve(&self, state: S) -> S;
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "event `{Self}` does not belong to bounded context `{BC}`",
    label = "event `{Self}` belongs to a different bounded context than `{BC}`",
    note = "a `DynamicConsistencyBoundary` or `ReadModel` can only consult or project events from its own `BoundedContext`"
)]
pub trait EventInBoundedContext<BC: BoundedContext>: architecturekit::Event {}

#[doc(hidden)]
pub const fn assert_event_in_bc<BC: BoundedContext, E: EventInBoundedContext<BC>>() {}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "event `{Self}` does not belong to domain `{D}`",
    label = "event `{Self}` belongs to a different domain than `{D}`",
    note = "domain-scoped integration components (`Policy`, `EventHandler`, `ProcessManager`) can only handle events from their own `Domain`"
)]
pub trait EventInDomain<D: crate::domain::Domain>: architecturekit::Event {}

#[doc(hidden)]
pub const fn assert_event_in_domain<D: crate::domain::Domain, E: EventInDomain<D>>() {}

#[doc(hidden)]
pub const fn assert_aggregate_event_scope<A: Aggregate>() {}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid event struct scope (`Aggregate` or `BoundedContext`)",
    label = "event structs must be scoped to an `Aggregate` or `BoundedContext`"
)]
pub trait EventStructScope: Kind {
    type BoundedContext: BoundedContext<Scope = Self::Domain>;
    type Domain: crate::domain::Domain;
}

#[doc(hidden)]
pub const fn assert_freestanding_event_scope<S: EventStructScope>() {}
