//! Tactical DDD and Event Sourcing building blocks scoped to a [`BoundedContext`].

pub mod actor;
pub mod aggregate;
pub mod command;
pub mod domain_service;
pub mod dynamic_consistency_boundary;
pub mod entity;
pub mod event;
pub mod query;
pub mod read_model;
pub mod value_object;

pub use crate::runtime::{CommandDescriptor, QueryDescriptor, ReadModelDescriptor};
pub use actor::{Actor, ActorType, Authorized};
pub use aggregate::Aggregate;
pub use command::Command;
pub use domain_service::DomainService;
pub use dynamic_consistency_boundary::DynamicConsistencyBoundary;
pub use entity::Entity;
pub use event::Event;
pub use query::Query;
pub use read_model::{DocumentView, Project, ReadModel};
pub use value_object::ValueObject;

use crate::{common::Kind, domain::Domain};

/// A self-contained linguistic and architectural boundary within a [`Domain`].
///
/// Tactical building blocks ([`Aggregate`], [`DynamicConsistencyBoundary`],
/// [`Entity`], [`ValueObject`], [`Event`], [`ReadModel`], [`Query`],
/// [`DomainService`], and [`Actor`]) are scoped to a [`BoundedContext`] and
/// inherit its topic prefix (`/<domain>/<bounded-context>`).
///
/// Derived via [`#[chromatofoor::bounded_context(scope = <Domain>)]`](chromatofoor_derive::bounded_context).
///
/// # Examples
///
/// ```rust
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// use chromatofoor::common::Kind;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Ordering;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Shipping;
///
/// assert_eq!(Ordering::KIND, "bounded-context");
/// assert_eq!(Ordering::TOPIC, "/commerce/ordering");
/// assert_eq!(Shipping::TOPIC, "/commerce/shipping");
/// ```
///
/// ## Compile-time scope check
///
/// A [`BoundedContext`] must be scoped to a [`Domain`] (not another [`BoundedContext`]):
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Ordering;
///
/// // Error: `Ordering` does not implement `Domain`
/// #[chromatofoor::bounded_context(scope = Ordering)]
/// pub struct Checkout;
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `BoundedContext`",
    label = "expected a `BoundedContext` here",
    note = "freestanding event structs (`#[event(scope = ...)] pub struct ...`) require a `BoundedContext` scope; for aggregate-scoped events, use `#[event(scope = <Aggregate>)] pub enum ...`"
)]
pub trait BoundedContext: Kind<Scope: Domain> {}
