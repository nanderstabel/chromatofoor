use crate::{bounded_context::BoundedContext, common::Kind};

/// Whether an [`Actor`] represents a human role or an automated system trigger.
///
/// Mirrors `actor.type` in the ESDM schema (`human` | `system`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorType {
    /// A person acting through a UI or API (end user, internal staff, partner contact).
    Human,
    /// An automated, domain-internal trigger (scheduler, recurring job, webhook bridge).
    System,
}

/// An ESDM `actor` — a ubiquitous-language role within a [`BoundedContext`] that
/// triggers commands or queries.
///
/// # Example
///
/// ```
/// # use chromatofoor::{bounded_context::{Actor, ActorType}, common::Kind};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::actor(scope = Ordering, type = human)]
/// pub struct Customer;
///
/// #[chromatofoor::actor(scope = Ordering, type = system)]
/// pub struct OrderTimeoutScheduler;
///
/// assert_eq!(Customer.topic().as_str(), "/commerce/ordering/customer");
/// assert_eq!(Customer::ACTOR_TYPE, ActorType::Human);
/// assert_eq!(OrderTimeoutScheduler::ACTOR_TYPE, ActorType::System);
/// ```
pub trait Actor: Kind<Scope: BoundedContext> {
    /// Whether this actor is a [`ActorType::Human`] or [`ActorType::System`] role.
    const ACTOR_TYPE: ActorType;

    /// Kebab-case names of the [`ExternalSystem`](crate::integration::ExternalSystem)s backing this
    /// system actor (`actor.backedBy` in ESDM). Always empty for [`ActorType::Human`] actors.
    const BACKED_BY: &'static [&'static str] = &[];
}

/// Actor-authorization metadata generated for commands and queries.
///
/// Commands and queries declare their allowed actors via `actors = [A1, A2]` or
/// `#[actors(A1, A2)]`.
///
/// # Example
///
/// ```
/// # use chromatofoor::{bounded_context::Authorized, common::Identifier};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering)]
/// # #[derive(Clone, Default)]
/// # pub struct Order;
/// # #[chromatofoor::event(scope = Order)]
/// # pub enum OrderEvent { OrderPlaced }
/// # impl chromatofoor::bounded_context::Event<Order> for OrderPlaced {
/// #     fn evolve(&self, state: Order) -> Order { state }
/// # }
/// #[chromatofoor::actor(scope = Ordering, type = human)]
/// pub struct Customer;
///
/// #[chromatofoor::actor(scope = Ordering, type = system)]
/// pub struct OrderTimeoutScheduler;
///
/// #[chromatofoor::command(scope = Order)]
/// pub enum OrderCommand {
///     #[lifecycle(create)]
///     #[actors(Customer)]
///     PlaceOrder,
///     #[actors(OrderTimeoutScheduler)]
///     ExpireOrder { id: Identifier },
///     InternalAudit { id: Identifier },
/// }
/// # impl chromatofoor::bounded_context::Command<Order> for PlaceOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for ExpireOrder {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
/// # impl chromatofoor::bounded_context::Command<Order> for InternalAudit {
/// #     type Error = std::convert::Infallible;
/// #     async fn decide(self, _: &Order) -> Result<Vec<OrderEvent>, Self::Error> { Ok(vec![]) }
/// # }
///
/// assert!(PlaceOrder::allows::<Customer>());
/// assert!(!PlaceOrder::allows::<OrderTimeoutScheduler>());
///
/// assert!(ExpireOrder::allows::<OrderTimeoutScheduler>());
/// assert!(!ExpireOrder::allows::<Customer>());
///
/// // Commands without declared actors fail closed (no access):
/// let audit = InternalAudit { id: "ord-1".into() };
/// assert!(!InternalAudit::allows::<Customer>());
/// assert!(!audit.is_authorized_for("customer"));
/// assert!(!audit.is_authorized_for_actor::<Customer>());
///
/// // Command enums enforce per-variant actor permissions:
/// let place_enum = OrderCommand::from(PlaceOrder {});
/// assert!(place_enum.is_authorized_for("customer"));
/// assert!(!place_enum.is_authorized_for("order-timeout-scheduler"));
/// ```
///
/// Referencing an actor from a different [`BoundedContext`] is rejected at compile time:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// #[chromatofoor::actor(scope = Billing, type = human)]
/// pub struct Accountant;
///
/// // Error: `Accountant` belongs to `Billing`, not `Ordering`
/// #[chromatofoor::query(scope = Ordering, actors = [Accountant])]
/// pub struct OrderById {
///     pub id: String,
/// }
/// ```
pub trait Authorized {
    /// Kebab-case name of the [`BoundedContext`] that scopes this command or query, when known.
    const BOUNDED_CONTEXT: &'static str = "";

    /// The `(name, ActorType)` pairs permitted to invoke this command or query.
    const ALLOWED_ACTORS: &'static [(&'static str, ActorType)] = &[];

    /// Returns `true` if `actor_name` (in kebab-case) is listed in [`Self::ALLOWED_ACTORS`].
    #[must_use]
    fn is_allowed_actor(actor_name: &str) -> bool {
        Self::ALLOWED_ACTORS
            .iter()
            .any(|(name, _)| *name == actor_name)
    }

    /// Returns `true` if the typed [`Actor`] `A` belongs to the same [`BoundedContext`] and is
    /// listed in [`Self::ALLOWED_ACTORS`] with a matching [`ActorType`].
    #[must_use]
    fn allows<A: Actor>() -> bool {
        (Self::BOUNDED_CONTEXT.is_empty()
            || Self::BOUNDED_CONTEXT == <A::Scope as crate::common::Kind>::NAME)
            && Self::ALLOWED_ACTORS
                .iter()
                .any(|(name, kind)| *name == A::NAME && *kind == A::ACTOR_TYPE)
    }

    /// Returns `true` if `actor_name` (in kebab-case) is permitted to invoke `self`.
    ///
    /// Authorization is fail-closed: when [`Self::ALLOWED_ACTORS`] is empty, no actor is
    /// permitted and this method returns `false`. Command enums generated by
    /// [`#[chromatofoor::command]`](crate::command) override this to delegate to the active variant.
    #[must_use]
    fn is_authorized_for(&self, actor_name: &str) -> bool {
        Self::is_allowed_actor(actor_name)
    }

    /// Returns `true` if the typed [`Actor`] `A` is permitted to invoke `self`.
    ///
    /// Authorization is fail-closed: when [`Self::ALLOWED_ACTORS`] is empty, no actor is
    /// permitted and this method returns `false`.
    #[must_use]
    fn is_authorized_for_actor<A: Actor>(&self) -> bool {
        Self::allows::<A>()
    }
}

#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "actor `{Self}` does not belong to bounded context `{BC}`",
    label = "actor `{Self}` belongs to a different bounded context than `{BC}`",
    note = "commands and queries can only authorize actors scoped to the same bounded context"
)]
pub trait ActorInBoundedContext<BC: BoundedContext>: Actor {}

#[doc(hidden)]
pub const fn assert_actor_in_bc<BC: BoundedContext, A: ActorInBoundedContext<BC>>() {}
