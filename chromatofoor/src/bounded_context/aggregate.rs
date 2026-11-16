use crate::{
    bounded_context::{BoundedContext, command::CommandScope},
    common::{Identified, Kind},
};

/// An ESDM `aggregate` — a persistent, container-based consistency unit within a
/// [`BoundedContext`].
///
/// # Identification Strategies (`identified_by`)
///
/// ## 1. `generated(uuid)` *(default)*
///
/// Creation commands (`#[lifecycle(create)]`) mint a fresh UUID subject; mutation commands
/// address an existing aggregate via `id: Identifier`:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified, Identifier}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering)]
/// #[derive(Clone, Default)]
/// pub struct Order;
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
///
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     PlaceOrder,
///     CancelOrder { id: Identifier },
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(Order::IDENTIFIED_BY, &[IdSource::Generated("uuid")]);
/// assert!((PlaceOrder {}).subject().as_str().starts_with("/commerce/ordering/order/"));
/// assert_eq!(
///     (CancelOrder { id: "550e8400-e29b-41d4-a716-446655440000".into() }).subject().as_str(),
///     "/commerce/ordering/order/550e8400-e29b-41d4-a716-446655440000"
/// );
/// ```
///
/// ## 2. `state(<field>)`
///
/// Identified by an [`Identifier`](crate::common::Identifier) field on the aggregate's state struct:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified, Identifier}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// #[derive(Clone, Default)]
/// pub struct Order {
///     pub order_id: Option<Identifier>,
/// }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
///
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     PlaceOrder { order_id: Identifier },
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(Order::IDENTIFIED_BY, &[IdSource::State("order_id")]);
/// assert_eq!(
///     (PlaceOrder { order_id: "ord-1".into() }).subject().as_str(),
///     "/commerce/ordering/order/ord-1"
/// );
/// ```
///
/// Referencing a nonexistent field in `identified_by = state(...)` is rejected at
/// compile time:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = state(missing_field))]
/// #[derive(Clone, Default)]
/// pub struct Order {
///     pub order_id: Option<Identifier>,
/// }
/// ```
///
/// ## 3. `static("<value>")`
///
/// Singleton aggregate within the bounded context addressed at a fixed subject suffix:
///
/// ```
/// # use chromatofoor::{architecturekit::Command as _, common::{IdSource, Identified}};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering, identified_by = static("singleton"))]
/// #[derive(Clone, Default)]
/// pub struct StorefrontSettings;
/// # #[chromatofoor::event(scope = StorefrontSettings)]
/// # pub enum StorefrontSettingsEvent { MaintenanceEnabled }
/// # impl chromatofoor::bounded_context::Event<StorefrontSettings> for MaintenanceEnabled {
/// #     fn evolve(&self, state: StorefrontSettings) -> StorefrontSettings { state }
/// # }
///
/// #[chromatofoor::command(scope = StorefrontSettings)]
/// pub enum StorefrontSettingsCommand {
///     EnableMaintenanceMode,
/// }
/// # impl chromatofoor::bounded_context::Command<StorefrontSettings> for EnableMaintenanceMode {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &StorefrontSettings) -> Result<Vec<StorefrontSettingsEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert_eq!(StorefrontSettings::IDENTIFIED_BY, &[IdSource::Static("singleton")]);
/// assert_eq!(
///     (EnableMaintenanceMode {}).subject().as_str(),
///     "/commerce/ordering/storefront-settings/singleton"
/// );
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an `Aggregate`",
    label = "expected an `Aggregate` here",
    note = "aggregate-scoped event enums (`#[event(scope = ...)] pub enum ...`) require an `Aggregate` scope; for freestanding bounded-context events, use `#[event(scope = <BoundedContext>)] pub struct ...`"
)]
pub trait Aggregate:
    Kind<Scope: BoundedContext> + Identified + CommandScope + architecturekit::State
{
}
