//! Runtime builder, catalog descriptors, and dynamic command/query dispatcher for presentation layers.
//!
//! While [`chromatofoor`](crate) provides static, zero-cost compile-time traits for domain logic,
//! presentation adapters (such as `chromatofoor-http`, `chromatofoor-graphql`, `chromatofoor-mcp`,
//! `chromatofoor-tui`, `chromatofoor-tauri`, or custom third-party transports) need a shared
//! runtime container that:
//!
//! 1. Starts and supervises all [`ReadModel`](crate::bounded_context::ReadModel) projections and
//!    [`EventConsumer`](crate::integration::EventConsumer)s (`Policy`, `EventHandler`,
//!    `ProcessManager`), waiting until projections catch up (`wait_caught_up`).
//! 2. Exposes a schema-rich catalog of registered [`CommandDescriptor`]s, [`QueryDescriptor`]s,
//!    [`ReadModelDescriptor`]s, and [`ConsumerDescriptor`]s derived directly from the Rust types
//!    and [`architecturekit::Schema`].
//! 3. Dynamically dispatches commands and queries from JSON payloads while enforcing
//!    [`Authorized`](crate::bounded_context::Authorized) actor checks.
//! 4. Attaches and runs one or more transport-agnostic [`Presentation`] layers via
//!    [`DomainBuilder::attach`] and [`DomainBuilder::serve`].
//!
//! # Example
//!
//! ```
//! use std::sync::Arc;
//! use chromatofoor::{
//!     architecturekit::{
//!         Envelope, InMemoryEventStore, InMemoryView, KeyedView, Store, ViewError,
//!     },
//!     bounded_context::{Command, Event, Project, Query},
//!     common::Identifier,
//!     domain::DomainBuilder,
//! };
//! use serde_json::json;
//!
//! #[chromatofoor::domain]
//! /// E-commerce problem space.
//! pub struct Commerce;
//!
//! #[chromatofoor::bounded_context(scope = Commerce)]
//! /// Order management context.
//! pub struct Ordering;
//!
//! #[chromatofoor::actor(scope = Ordering, type = human)]
//! pub struct Customer;
//!
//! #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
//! #[derive(Clone, Default)]
//! pub struct Order {
//!     pub order_id: Option<Identifier>,
//!     pub total_cents: u64,
//! }
//!
//! #[chromatofoor::event(scope = Order)]
//! pub enum OrderEvent {
//!     OrderPlaced { order_id: Identifier, total_cents: u64 },
//! }
//!
//! impl Event<Order> for OrderPlaced {
//!     fn evolve(&self, _state: Order) -> Order {
//!         Order {
//!             order_id: Some(self.order_id.clone()),
//!             total_cents: self.total_cents,
//!         }
//!     }
//! }
//!
//! #[chromatofoor::command(scope = Order)]
//! pub enum OrderCommand {
//!     /// Places a new customer order.
//!     #[lifecycle(create)]
//!     #[actors(Customer)]
//!     PlaceOrder { order_id: Identifier, total_cents: u64 },
//! }
//!
//! impl Command<Order> for PlaceOrder {
//!     type Error = &'static str;
//!
//!     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
//!         Ok(vec![OrderPlaced {
//!             order_id: self.order_id,
//!             total_cents: self.total_cents,
//!         }
//!         .into()])
//!     }
//! }
//!
//! #[chromatofoor::read_model(scope = Ordering, events = [OrderPlaced])]
//! pub struct OrderTotals {
//!     pub totals: InMemoryView<String, (String, u64)>,
//! }
//!
//! impl Project<OrderPlaced> for OrderTotals {
//!     async fn project(&self, event: Envelope<OrderPlaced>) -> Result<(), ViewError> {
//!         self.totals
//!             .insert(&event, (event.data.order_id.to_string(), event.data.total_cents))?;
//!         Ok(())
//!     }
//! }
//!
//! /// Looks up the total for a single order.
//! #[chromatofoor::query(scope = Ordering, actors = [Customer])]
//! pub struct OrderTotalById {
//!     pub order_id: String,
//! }
//!
//! impl Query<OrderTotals> for OrderTotalById {
//!     type Output = Option<u64>;
//!     type Error = std::convert::Infallible;
//!
//!     async fn execute(self, read_model: &OrderTotals) -> Result<Self::Output, Self::Error> {
//!         let entry = read_model.totals.get(&self.order_id).await?;
//!         Ok(entry.map(|(_, cents)| cents))
//!     }
//! }
//!
//! # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
//! let store = Arc::new(
//!     Store::builder(InMemoryEventStore::new(), "https://commerce.local").build(),
//! );
//!
//! let order_totals = Arc::new(OrderTotals {
//!     totals: InMemoryView::new(|(id, _): &(String, u64)| id.clone()),
//! });
//!
//! let runtime = DomainBuilder::<Commerce>::new(store)
//!     .register_command::<PlaceOrder, _>()
//!     .register_read_model(Arc::clone(&order_totals))
//!     .register_query::<OrderTotalById, _>(Arc::clone(&order_totals))
//!     .build()
//!     .await
//!     .unwrap();
//!
//! // Inspect schema-derived catalog:
//! assert_eq!(runtime.domain_name(), "commerce");
//! assert_eq!(runtime.commands().len(), 1);
//! assert_eq!(runtime.commands()[0].name, "place-order");
//! assert_eq!(runtime.commands()[0].description, "Places a new customer order.");
//! assert_eq!(
//!     runtime.commands()[0].schema["required"],
//!     json!(["order_id", "total_cents"])
//! );
//!
//! // Dynamic command execution with actor check:
//! let events = runtime
//!     .dispatch_command(
//!         "ordering",
//!         "order",
//!         "place-order",
//!         json!({ "order_id": "ord-1", "total_cents": 4200 }),
//!         Some("customer"),
//!     )
//!     .await
//!     .unwrap();
//! assert_eq!(events.len(), 1);
//!
//! // Dynamic query execution after waiting for projections to catch up:
//! runtime.wait_caught_up().await.unwrap();
//! let result = runtime
//!     .dispatch_query(
//!         "ordering",
//!         "order-total-by-id",
//!         json!({ "order_id": "ord-1" }),
//!         Some("customer"),
//!     )
//!     .await
//!     .unwrap();
//! assert_eq!(result, json!(4200));
//! # });
//! ```

mod descriptor;
mod domain;
mod error;

pub use crate::common::{Problem, ProblemDetails};
pub use descriptor::{CommandDescriptor, ConsumerDescriptor, QueryDescriptor, ReadModelDescriptor};
pub use domain::{DomainBuilder, DomainRuntime, Presentation};
pub use error::DispatchError;
