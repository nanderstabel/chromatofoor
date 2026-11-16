use crate::common::{Identified, Kind};
use std::future::Future;

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid command scope (`Aggregate` or `DynamicConsistencyBoundary`)",
    label = "commands must be scoped to an `Aggregate` or `DynamicConsistencyBoundary`",
    note = "annotate `{Self}` with `#[chromatofoor::aggregate(...)]` or `#[chromatofoor::dynamic_consistency_boundary(...)]`"
)]
pub trait CommandScope:
    architecturekit::State<Event: Send + Sync + 'static> + Kind + Identified + Send + Sync
{
    /// Executes `command` against `store` using this scope's execution strategy
    /// (`Store::execute` for [`Aggregate`](crate::bounded_context::Aggregate), or
    /// [`DynamicConsistencyBoundary::execute`](crate::bounded_context::DynamicConsistencyBoundary::execute)).
    fn execute<C: Command<Self>>(
        store: &architecturekit::Store,
        command: C,
    ) -> impl Future<
        Output = Result<
            Vec<architecturekit::eventsourcingdb::Event>,
            architecturekit::Error<C::Error>,
        >,
    > + Send;
}

#[doc(hidden)]
pub const fn assert_command_scope<S: CommandScope>() {}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "command `{Self}` does not belong to domain `{D}`",
    label = "command `{Self}` belongs to a different domain than `{D}`",
    note = "domain-scoped integration components (`Policy`, `ProcessManager`) can only emit commands belonging to their own `Domain`"
)]
pub trait CommandInDomain<D: crate::domain::Domain>: architecturekit::Command {}

#[doc(hidden)]
pub const fn assert_command_in_domain<D: crate::domain::Domain, C: CommandInDomain<D>>() {}

#[doc(hidden)]
pub trait ValidatePayload<E> {
    fn validate_payload(&self) -> Result<(), E> {
        Ok(())
    }
}

/// An ESDM `command` — an intention to change state within an
/// [`Aggregate`](crate::bounded_context::Aggregate) or
/// [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary).
///
/// Commands are declared as variants of an `enum` annotated with
/// [`#[chromatofoor::command(scope = ...)]`](crate::command). Each variant expands into a
/// standalone struct implementing [`Kind`], [`Authorized`](crate::bounded_context::Authorized),
/// and [`architecturekit::Command`], and the enum itself implements `Command<S>`.
///
/// # Example
///
/// ```
/// # use chromatofoor::{
/// #     architecturekit::{Precondition, Subject, testing},
/// #     bounded_context::{Command, Event},
/// #     common::Identifier,
/// # };
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier>, pub is_canceled: bool }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced { order_id: Identifier }, OrderCanceled }
/// # impl Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { Order { order_id: Some(self.order_id.clone()), ..state } }
/// # }
/// # impl Event<Order> for OrderCanceled {
/// #     fn evolve(&self, state: Order) -> Order { Order { is_canceled: true, ..state } }
/// # }
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     PlaceOrder { order_id: Identifier },
///     CancelOrder { order_id: Identifier },
/// }
///
/// impl Command<Order> for PlaceOrder {
///     type Error = &'static str;
///
///     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
///         Ok(vec![OrderPlaced { order_id: self.order_id }.into()])
///     }
/// }
///
/// impl Command<Order> for CancelOrder {
///     type Error = &'static str;
///
///     async fn decide(self, state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
///         if state.is_canceled {
///             return Err("already canceled");
///         }
///         Ok(vec![OrderCanceled {}.into()])
///     }
/// }
///
/// # tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
/// testing::given(&OrderCommand::decider(), [OrderPlaced { order_id: "ord-1".into() }.into()])
///     .when(CancelOrder { order_id: "ord-1".into() }.into())
///     .await
///     .then_events([OrderCanceled {}.into()])
///     .then_preconditions([
///         Precondition::on_populated_subject(Subject::new("/commerce/ordering/order/ord-1")),
///         Precondition::on_state_read(),
///     ]);
/// # });
/// ```
///
/// Using `#[chromatofoor::command(..., validate)]` automatically derives
/// `chromatofoor::validator::Validate` (with `#[validate(crate = "chromatofoor::validator")]`)
/// without requiring a direct `validator` dependency or `use chromatofoor::validator::Validate;`:
///
/// ```
/// pub mod commerce {
///     #[chromatofoor::domain]
///     pub struct Commerce;
///
///     pub mod ordering {
///         #[chromatofoor::bounded_context(scope = super::Commerce)]
///         pub struct Ordering;
///
///         #[chromatofoor::value_object(scope = Ordering)]
///         #[derive(chromatofoor::validator::Validate)]
///         pub struct CancellationNote {
///             #[validate(length(min = 2))]
///             pub note: String,
///         }
///
///         #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
///         #[derive(Clone, Default)]
///         pub struct Order {
///             pub order_id: Option<chromatofoor::Identifier>,
///         }
///
///         #[chromatofoor::event(scope = Order)]
///         pub enum OrderEvent {
///             OrderPlaced { order_id: chromatofoor::Identifier },
///         }
///
///         impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
///             fn evolve(&self, _state: Order) -> Order {
///                 Order {
///                     order_id: Some(self.order_id.clone()),
///                 }
///             }
///         }
///
///         #[derive(Debug, thiserror::Error)]
///         pub enum OrderError {
///             #[error(transparent)]
///             Validation(#[from] chromatofoor::validator::ValidationErrors),
///         }
///
///         #[chromatofoor::command(scope = Order, validate)]
///         pub struct CancelOrder {
///             pub order_id: chromatofoor::Identifier,
///             #[validate(length(min = 3))]
///             pub reason: String,
///             #[validate(nested)]
///             pub detail: Option<CancellationNote>,
///         }
///
///         impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
///             type Error = OrderError;
///             async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
///                 Ok(vec![])
///             }
///         }
///
///         #[chromatofoor::command(scope = Order, validate)]
///         pub enum OrderCommand {
///             ArchiveOrder {
///                 order_id: chromatofoor::Identifier,
///                 #[validate(length(min = 3))]
///                 reason: String,
///             },
///         }
///
///         impl chromatofoor::bounded_context::Command<Order> for ArchiveOrder {
///             type Error = OrderError;
///             async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
///                 Ok(vec![])
///             }
///         }
///     }
/// }
///
/// use commerce::ordering;
/// use chromatofoor::validator::Validate as _;
/// let bad = ordering::CancelOrder {
///     order_id: "ord-1".into(),
///     reason: "no".to_string(),
///     detail: None,
/// };
/// assert!(bad.validate().is_err());
///
/// let bad_nested = ordering::CancelOrder {
///     order_id: "ord-1".into(),
///     reason: "valid reason".to_string(),
///     detail: Some(ordering::CancellationNote {
///         note: "x".to_string(),
///     }),
/// };
/// assert!(bad_nested.validate().is_err());
///
/// let good = ordering::CancelOrder {
///     order_id: "ord-1".into(),
///     reason: "valid reason".to_string(),
///     detail: Some(ordering::CancellationNote {
///         note: "ok".to_string(),
///     }),
/// };
/// assert!(good.validate().is_ok());
/// ```
///
/// # Compile-Time Guarantees
///
/// Applying `#[chromatofoor::command]` with a non-command scope (`Domain` or `BoundedContext`)
/// is rejected at compile time:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::command(scope = Ordering, creates)]
/// pub struct PlaceOrder;
/// ```
///
/// Omitting a field required by `S::IDENTIFIED_BY` is also rejected at compile time:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering, identified_by = state(order_id))]
/// # #[derive(Clone, Default)]
/// # pub struct Order { pub order_id: Option<Identifier> }
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderCanceled }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderCanceled {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     // Missing `order_id: Identifier` required by `identified_by = state(order_id)`:
///     CancelOrder {},
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for CancelOrder {
/// #     type Error = &'static str;
/// #     async fn decide(self, _state: &Order) -> Result<Vec<OrderEvent>, Self::Error> {
/// #         Ok(vec![OrderCanceled {}.into()])
/// #     }
/// # }
/// ```
pub trait Command<S: CommandScope>:
    Kind<Scope = S>
    + crate::bounded_context::Authorized
    + ValidatePayload<Self::Error>
    + architecturekit::Command
    + architecturekit::Schema
    + serde::de::DeserializeOwned
    + Clone
    + Send
    + Sync
    + Sized
{
    /// The domain error returned when the command violates an invariant of `S`.
    type Error: std::fmt::Display + Send + 'static;

    /// Validates business rules against `state` and returns the domain events to record.
    fn decide(self, state: &S) -> impl Future<Output = Result<Vec<S::Event>, Self::Error>> + Send;

    /// Returns whether a consulted event envelope should be folded into `S` when executing
    /// this command on a [`DynamicConsistencyBoundary`](crate::bounded_context::DynamicConsistencyBoundary).
    ///
    /// Defaults to `true`. Override to filter consulted events by [`Envelope::subject`](architecturekit::Envelope::subject)
    /// or payload fields when implementing per-instance `consults` criteria.
    fn matches(&self, _event: &architecturekit::Envelope<S::Event>) -> bool {
        true
    }

    /// Wraps [`Command::decide`] into an [`architecturekit::Decider`] for execution with
    /// [`architecturekit::Store::execute`] or [`architecturekit::testing::given`].
    #[must_use]
    fn decider() -> architecturekit::Decider<Self, S, Self::Error>
    where
        Self: Send + 'static,
        S: Send + 'static,
        S::Event: Send + 'static,
        Self::Error: Send + 'static,
    {
        architecturekit::Decider::new(|cmd: Self, state: S| async move {
            cmd.validate_payload()?;
            cmd.decide(&state).await
        })
    }
}
