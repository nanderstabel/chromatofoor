use crate::{bounded_context::ReadModel, common::Kind};
use std::future::Future;

/// An ESDM `query` — a typed, read-only request that retrieves data from a [`ReadModel`] in
/// the same [`BoundedContext`](crate::bounded_context::BoundedContext).
///
/// [`#[chromatofoor::query(scope = ..., actors = [...])]`](crate::query) implements [`Kind`]
/// and [`Authorized`](crate::bounded_context::Authorized) for the query struct. Implementing
/// `Query<R>` enforces at compile time (`Kind<Scope = R::Scope>`) that the query and the
/// target [`ReadModel`] `R` belong to the same bounded context.
///
/// # Example
///
/// ```
/// # use chromatofoor::{
/// #     architecturekit::{Envelope, InMemoryView, KeyedView, Subject, ViewError, testing},
/// #     bounded_context::{Authorized, Project, Query},
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::actor(scope = Ordering, type = human)]
/// # pub struct Customer;
/// # #[chromatofoor::event(scope = Ordering)]
/// # pub struct OrderPlaced { pub order_id: String, pub total_cents: u64 }
/// # #[chromatofoor::read_model(scope = Ordering, events = [OrderPlaced])]
/// # pub struct OrderTotals { pub totals: InMemoryView<String, (String, u64)> }
/// # impl Project<OrderPlaced> for OrderTotals {
/// #     async fn project(&self, event: Envelope<OrderPlaced>) -> Result<(), ViewError> {
/// #         self.totals.insert(&event, (event.data.order_id.clone(), event.data.total_cents))?;
/// #         Ok(())
/// #     }
/// # }
/// #[chromatofoor::query(scope = Ordering, actors = [Customer])]
/// pub struct OrderTotalById {
///     pub order_id: String,
/// }
///
/// impl Query<OrderTotals> for OrderTotalById {
///     type Output = Option<u64>;
///     type Error = std::convert::Infallible;
///
///     async fn execute(self, read_model: &OrderTotals) -> Result<Self::Output, Self::Error> {
///         let entry = read_model.totals.get(&self.order_id).await?;
///         Ok(entry.map(|(_, cents)| cents))
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// assert!(OrderTotalById::allows::<Customer>());
///
/// let read_model = OrderTotals {
///     totals: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
/// };
/// # testing::project(
/// #     &read_model,
/// #     &testing::stored_events(
/// #         &Subject::new("/commerce/ordering/order/ord-1"),
/// #         [OrderTotalsEvents::from(OrderPlaced { order_id: "ord-1".to_owned(), total_cents: 2500 })],
/// #     ),
/// # ).await.unwrap();
///
/// let total = OrderTotalById { order_id: "ord-1".to_owned() }
///     .execute(&read_model)
///     .await
///     .unwrap();
/// assert_eq!(total, Some(2500));
/// # });
/// ```
///
/// Querying a [`ReadModel`] from a different bounded context is rejected at compile time:
///
/// ```compile_fail
/// # use chromatofoor::bounded_context::Query;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Shipping;
/// #[chromatofoor::event(scope = Shipping)]
/// pub struct OrderShipped;
///
/// #[chromatofoor::read_model(scope = Shipping, events = [OrderShipped])]
/// pub struct ShipmentTracker;
/// # impl chromatofoor::bounded_context::Project<OrderShipped> for ShipmentTracker {
/// #     async fn project(&self, _event: chromatofoor::architecturekit::Envelope<OrderShipped>) -> Result<(), chromatofoor::architecturekit::ViewError> {
/// #         Ok(())
/// #     }
/// # }
///
/// #[chromatofoor::query(scope = Ordering)]
/// pub struct ShipmentByOrderId {
///     pub order_id: String,
/// }
///
/// // Error: `ShipmentByOrderId` is scoped to `Ordering`, while `ShipmentTracker` is scoped to `Shipping`
/// impl Query<ShipmentTracker> for ShipmentByOrderId {
///     type Output = Option<String>;
///     type Error = std::convert::Infallible;
///
///     async fn execute(self, _read_model: &ShipmentTracker) -> Result<Self::Output, Self::Error> {
///         Ok(None)
///     }
/// }
/// ```
pub trait Query<R: ReadModel>:
    Kind<Scope = R::Scope>
    + crate::bounded_context::Authorized
    + architecturekit::Schema
    + serde::de::DeserializeOwned
    + Send
    + Sync
    + Sized
{
    /// The result returned when the query succeeds.
    type Output: architecturekit::Schema + serde::Serialize + Send + 'static;
    /// The error returned if querying the read model fails.
    type Error: std::fmt::Display + Send + 'static;

    /// Executes this query against `read_model`.
    fn execute(
        self,
        read_model: &R,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send;
}
