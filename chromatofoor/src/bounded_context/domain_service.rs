//! Stateless domain logic scoped to a [`BoundedContext`].

use crate::{bounded_context::BoundedContext, common::Kind};

/// A stateless service within a [`BoundedContext`] that encapsulates pure domain
/// logic or calculations that do not naturally belong to a single
/// [`Aggregate`](crate::bounded_context::Aggregate) or
/// [`ValueObject`](crate::bounded_context::ValueObject).
///
/// Derived via [`#[chromatofoor::domain_service(scope = <BoundedContext>)]`](chromatofoor_derive::domain_service).
///
/// # Examples
///
/// ```rust
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// # #[chromatofoor::value_object(scope = Ordering)]
/// # pub struct Money {
/// #     pub cents: u64,
/// # }
/// use chromatofoor::common::Kind;
///
/// #[chromatofoor::domain_service(scope = Ordering)]
/// pub struct PricingCalculator;
///
/// impl PricingCalculator {
///     pub fn apply_discount(subtotal: &Money, discount_percent: u64) -> Money {
///         Money {
///             cents: subtotal.cents * (100 - discount_percent) / 100,
///         }
///     }
/// }
///
/// assert_eq!(PricingCalculator::TOPIC, "/commerce/ordering/pricing-calculator");
/// assert_eq!(
///     PricingCalculator::apply_discount(&Money { cents: 2000 }, 15),
///     Money { cents: 1700 }
/// );
/// ```
///
/// ## Compile-time scope check
///
/// A [`DomainService`] must be scoped to a [`BoundedContext`] (not an [`Aggregate`](crate::bounded_context::Aggregate)):
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::aggregate(scope = Ordering)]
/// #[derive(Clone, Default)]
/// pub struct Order;
///
/// // Error: `Order` does not implement `BoundedContext`
/// #[chromatofoor::domain_service(scope = Order)]
/// pub struct PricingCalculator;
/// ```
pub trait DomainService: Kind<Scope: BoundedContext> {}
