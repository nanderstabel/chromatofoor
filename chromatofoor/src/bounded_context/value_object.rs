use crate::{bounded_context::BoundedContext, common::Kind};

/// An ESDM `value-object` — an immutable, identity-less domain value within a
/// [`BoundedContext`] defined solely by its attributes (`Money`, `ShippingAddress`).
///
/// [`#[chromatofoor::value_object(scope = ...)]`](crate::value_object) derives `Clone`,
/// `Debug`, `PartialEq`, `serde::Serialize`, `serde::Deserialize`, and
/// [`architecturekit::Schema`], and implements [`Kind`] and [`ValueObject`].
///
/// # Example
///
/// ```
/// # use chromatofoor::common::Kind;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::value_object(scope = Ordering)]
/// pub struct Money {
///     pub amount_cents: u64,
///     pub currency: String,
/// }
///
/// let a = Money { amount_cents: 2500, currency: "EUR".to_owned() };
/// let b = Money { amount_cents: 2500, currency: "EUR".to_owned() };
///
/// assert_eq!(a, b);
/// assert_eq!(a.topic().as_str(), "/commerce/ordering/money");
/// ```
///
/// Value objects must be scoped to a [`BoundedContext`]:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::aggregate(scope = Ordering)]
/// # #[derive(Clone, Default)]
/// # pub struct Order;
/// // Error: `Order` is an `Aggregate`, not a `BoundedContext`
/// #[chromatofoor::value_object(scope = Order)]
/// pub struct Money {
///     pub amount_cents: u64,
/// }
/// ```
pub trait ValueObject: Kind<Scope: BoundedContext> + architecturekit::Schema {}
