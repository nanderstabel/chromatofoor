pub use crate::runtime::{
    ConsumerDescriptor, DispatchError, DomainBuilder, DomainRuntime, Presentation,
};
use crate::{bounded_context::BoundedContext, common::Kind};

/// The top-level problem space (`kind: domain`) that owns [`BoundedContext`]s,
/// strategic [`Subdomain`] classifications, and cross-context integration components
/// ([`Policy`](crate::integration::Policy), [`EventHandler`](crate::integration::EventHandler),
/// [`ProcessManager`](crate::integration::ProcessManager), [`ExternalSystem`](crate::integration::ExternalSystem)).
///
/// Use [`#[chromatofoor::domain]`](chromatofoor_derive::domain) to declare a domain marker type
/// whose [`Kind::TOPIC`] forms the root subject prefix (`/<domain>`).
///
/// # Examples
///
/// ```
/// use chromatofoor::common::{Kind, Topic};
///
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// assert_eq!(Commerce::KIND, "domain");
/// assert_eq!(Commerce::NAME, "commerce");
/// assert_eq!(Commerce::TOPIC, "/commerce");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Domain`",
    label = "expected a `Domain` here",
    note = "mark `{Self}` with `#[chromatofoor::domain]`"
)]
pub trait Domain: Kind<Scope = ()> {}

/// Strategic classification of a [`Subdomain`] by competitive value (`core`, `supporting`, or `generic`).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr, strum::Display, strum::IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SubdomainType {
    /// Competitively differentiating capability; the product's reason to exist.
    Core,
    /// Necessary for the business and specific enough to build in-house, but not differentiating.
    Supporting,
    /// Commodity problem with well-known solutions (payments, auth, notifications) typically bought or outsourced.
    Generic,
}

/// Compile-time witness that a [`BoundedContext`] belongs to [`Domain`] `D`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `BoundedContext` scoped to domain `{D}`",
    label = "expected a `BoundedContext` in domain `{D}`",
    note = "every bounded context in `#[subdomain(scope = ..., bounded_contexts = [...])]` must be marked with `#[chromatofoor::bounded_context(scope = {D})]`"
)]
pub trait BoundedContextInDomain<D: Domain>: BoundedContext {}

impl<D, B> BoundedContextInDomain<D> for B
where
    D: Domain,
    B: BoundedContext + Kind<Scope = D>,
{
}

/// Compile-time assertion helper used by [`#[chromatofoor::subdomain]`](chromatofoor_derive::subdomain).
pub const fn assert_bounded_context_in_domain<D: Domain, B: BoundedContextInDomain<D>>() {}

/// Strategic classification of a portion of a [`Domain`] (`kind: subdomain`).
///
/// A [`Subdomain`] is orthogonal to [`BoundedContext`] rather than a hierarchical layer above it:
/// bounded contexts remain scoped directly to their [`Domain`] (`/<domain>/<bounded-context>`),
/// while a [`Subdomain`] groups one or more [`BoundedContext`]s by strategic investment
/// ([`SubdomainType::Core`], [`SubdomainType::Supporting`], or [`SubdomainType::Generic`]).
///
/// Derived via [`#[chromatofoor::subdomain(scope = <Domain>, type = core | supporting | generic, bounded_contexts = [...])]`](chromatofoor_derive::subdomain).
///
/// # Examples
///
/// ```
/// use chromatofoor::{
///     common::Kind,
///     domain::{Subdomain, SubdomainType},
/// };
///
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Ordering;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Inventory;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Shipping;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Billing;
///
/// #[chromatofoor::subdomain(
///     scope = Commerce,
///     type = core,
///     bounded_contexts = [Ordering, Inventory],
/// )]
/// pub struct OrderFulfillment;
///
/// #[chromatofoor::subdomain(
///     scope = Commerce,
///     type = supporting,
///     bounded_contexts = [Shipping],
/// )]
/// pub struct Logistics;
///
/// #[chromatofoor::subdomain(
///     scope = Commerce,
///     type = generic,
///     bounded_contexts = [Billing],
/// )]
/// pub struct Payments;
///
/// assert_eq!(OrderFulfillment::KIND, "subdomain");
/// assert_eq!(OrderFulfillment::NAME, "order-fulfillment");
/// assert_eq!(OrderFulfillment::TOPIC, "/commerce/order-fulfillment");
/// assert_eq!(OrderFulfillment::SUBDOMAIN_TYPE, SubdomainType::Core);
/// assert_eq!(OrderFulfillment::SUBDOMAIN_TYPE.as_ref(), "core");
/// assert_eq!(OrderFulfillment::BOUNDED_CONTEXTS, &["ordering", "inventory"]);
///
/// // Bounded contexts remain scoped directly to the Domain (`"/commerce/..."`):
/// assert_eq!(Ordering::TOPIC, "/commerce/ordering");
/// assert_eq!(Logistics::SUBDOMAIN_TYPE, SubdomainType::Supporting);
/// assert_eq!(Payments::SUBDOMAIN_TYPE, SubdomainType::Generic);
/// ```
///
/// ## Compile-time verification
///
/// Every entry in `bounded_contexts = [...]` must be a [`BoundedContext`] scoped to the same [`Domain`]:
///
/// ```compile_fail
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// #[chromatofoor::domain]
/// pub struct Marketing;
///
/// #[chromatofoor::bounded_context(scope = Marketing)]
/// pub struct Campaigns;
///
/// // Error: `Campaigns` is scoped to `Marketing`, not `Commerce`
/// #[chromatofoor::subdomain(
///     scope = Commerce,
///     type = supporting,
///     bounded_contexts = [Campaigns],
/// )]
/// pub struct Promotions;
/// ```
///
/// And `bounded_contexts` must contain at least one [`BoundedContext`]:
///
/// ```compile_fail
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// // Error: `bounded_contexts` must contain at least one bounded context
/// #[chromatofoor::subdomain(
///     scope = Commerce,
///     type = core,
///     bounded_contexts = [],
/// )]
/// pub struct EmptySubdomain;
/// ```
pub trait Subdomain: Kind<Scope: Domain> {
    /// Strategic classification (`Core`, `Supporting`, or `Generic`).
    const SUBDOMAIN_TYPE: SubdomainType;

    /// Bare kebab-case names of the [`BoundedContext`]s within [`Self::Scope`](Kind::Scope)
    /// that this subdomain covers.
    const BOUNDED_CONTEXTS: &'static [&'static str];
}
