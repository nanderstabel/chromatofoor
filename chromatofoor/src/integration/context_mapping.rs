//! Strategic relationships (`kind: context-mapping`) between [`BoundedContext`]s
//! and [`ExternalSystem`](crate::integration::ExternalSystem)s.

use crate::{bounded_context::BoundedContext, common::Kind};

/// Marker trait for valid endpoints of an asymmetric [`ContextMapping`]
/// (`mappingEndpoint` in ESDM): either a [`BoundedContext`]
/// or an [`ExternalSystem`](crate::integration::ExternalSystem).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a `context-mapping` endpoint",
    label = "expected a `#[bounded_context]` or `#[external_system]` type here"
)]
pub trait MappingEndpoint: Kind {}

/// Strategic relationship pattern and role endpoints for a [`ContextMapping`]
/// (`context-mapping` in ESDM).
///
/// Endpoint strings hold each participant's [`Kind::TOPIC`] (`/<domain>/<bounded-context>`
/// or `/<domain>/<external-system>`).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, strum::AsRefStr, strum::Display, strum::IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum ContextMappingType {
    /// Upstream `supplier` prioritizes downstream `customer` needs (`customer-supplier`).
    CustomerSupplier {
        /// Topic of the downstream customer endpoint.
        customer: &'static str,
        /// Topic of the upstream supplier endpoint.
        supplier: &'static str,
    },
    /// Downstream `conformist` adopts the `upstream` model without translation (`conformist`).
    Conformist {
        /// Topic of the conforming downstream endpoint.
        conformist: &'static str,
        /// Topic of the upstream endpoint.
        upstream: &'static str,
    },
    /// `downstream` translates the `upstream` model via a boundary adapter (`anti-corruption-layer`).
    AntiCorruptionLayer {
        /// Topic of the downstream endpoint owning the translation layer.
        downstream: &'static str,
        /// Topic of the upstream endpoint being translated.
        upstream: &'static str,
    },
    /// `host` exposes a standardized protocol/API consumed by `consumer` (`open-host-service`).
    OpenHostService {
        /// Topic of the hosting endpoint.
        host: &'static str,
        /// Topic of the consuming endpoint.
        consumer: &'static str,
    },
    /// `publisher` and `consumer` communicate via a shared wire schema (`published-language`).
    PublishedLanguage {
        /// Topic of the publishing endpoint.
        publisher: &'static str,
        /// Topic of the consuming endpoint.
        consumer: &'static str,
    },
    /// Two [`BoundedContext`]s share a common subset of internal domain types (`shared-kernel`).
    SharedKernel {
        /// Topics of the two participating bounded contexts.
        participants: [&'static str; 2],
    },
    /// Two [`BoundedContext`]s coordinate features and deployments in lockstep (`partnership`).
    Partnership {
        /// Topics of the two participating bounded contexts.
        participants: [&'static str; 2],
    },
    /// Two [`BoundedContext`]s explicitly choose not to integrate (`separate-ways`).
    SeparateWays {
        /// Topics of the two participating bounded contexts.
        participants: [&'static str; 2],
    },
}

/// Top-level strategic relationship (`kind: context-mapping`, `Scope = ()`) between two
/// [`BoundedContext`]s or between a [`BoundedContext`] and an
/// [`ExternalSystem`](crate::integration::ExternalSystem).
///
/// Unlike other ESDM kinds, `context-mapping` has no enclosing `scope` because a mapping
/// may span bounded contexts in different domains or link a bounded context to an external
/// system:
/// - **Asymmetric mappings** (`customer_supplier`, `conformist`, `anti_corruption_layer`,
///   `open_host_service`, `published_language`) accept any [`MappingEndpoint`]
///   ([`BoundedContext`] or [`ExternalSystem`](crate::integration::ExternalSystem)) on each side.
/// - **Symmetric mappings** (`shared_kernel`, `partnership`, `separate_ways`) require
///   `participants = [<BoundedContext>, <BoundedContext>]`.
///
/// # Examples
///
/// ```
/// use chromatofoor::{
///     common::{Kind, Topic},
///     integration::{ContextMapping, ContextMappingType},
/// };
///
/// #[chromatofoor::domain]
/// pub struct Commerce;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Ordering;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Billing;
///
/// #[chromatofoor::bounded_context(scope = Commerce)]
/// pub struct Shipping;
///
/// #[chromatofoor::external_system(
///     scope = Commerce,
///     direction = bidirectional,
///     category = "payment",
/// )]
/// pub struct Stripe;
///
/// // 1. `Shipping` (customer) depends on `Ordering` (supplier) for order fulfillment details:
/// #[chromatofoor::context_mapping(
///     type = customer_supplier,
///     customer = Shipping,
///     supplier = Ordering,
/// )]
/// pub struct OrderToShippingSupply;
///
/// // 2. `Billing` translates `Stripe`'s external API via an Anti-Corruption Layer:
/// #[chromatofoor::context_mapping(
///     type = anti_corruption_layer,
///     downstream = Billing,
///     upstream = Stripe,
/// )]
/// pub struct StripeBillingAcl;
///
/// // 3. Symmetric `shared_kernel` between `Ordering` and `Billing` (both BoundedContexts):
/// #[chromatofoor::context_mapping(
///     type = shared_kernel,
///     participants = [Ordering, Billing],
/// )]
/// pub struct SharedMoneyKernel;
///
/// assert_eq!(OrderToShippingSupply::KIND, "context-mapping");
/// assert_eq!(
///     OrderToShippingSupply.topic().as_str(),
///     "/order-to-shipping-supply"
/// );
/// assert_eq!(
///     OrderToShippingSupply::MAPPING,
///     ContextMappingType::CustomerSupplier {
///         customer: "/commerce/shipping",
///         supplier: "/commerce/ordering",
///     }
/// );
/// assert_eq!(
///     StripeBillingAcl::MAPPING,
///     ContextMappingType::AntiCorruptionLayer {
///         downstream: "/commerce/billing",
///         upstream: "/commerce/stripe",
///     }
/// );
/// assert_eq!(
///     SharedMoneyKernel::MAPPING,
///     ContextMappingType::SharedKernel {
///         participants: ["/commerce/ordering", "/commerce/billing"],
///     }
/// );
/// ```
///
/// ## Compile-time checks
///
/// 1. Symmetric mappings (`shared_kernel`, `partnership`, `separate_ways`) reject
///    [`ExternalSystem`](crate::integration::ExternalSystem)s — you cannot share a kernel or
///    form a partnership with a third-party external system:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Billing;
/// # #[chromatofoor::external_system(scope = Commerce, direction = bidirectional)]
/// # pub struct Stripe;
/// // Error: `Stripe` is an `ExternalSystem`, not a `BoundedContext`
/// #[chromatofoor::context_mapping(
///     type = shared_kernel,
///     participants = [Billing, Stripe],
/// )]
/// pub struct InvalidExternalSharedKernel;
/// ```
///
/// 2. Mapping an endpoint to itself is rejected at compile time:
///
/// ```compile_fail
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// // Error: a context mapping must link two distinct endpoints
/// #[chromatofoor::context_mapping(
///     type = customer_supplier,
///     customer = Ordering,
///     supplier = Ordering,
/// )]
/// pub struct SelfMapping;
/// ```
pub trait ContextMapping: Kind<Scope = ()> {
    /// The pattern type and endpoints of this context mapping.
    const MAPPING: ContextMappingType;
}

#[doc(hidden)]
pub const fn assert_mapping_endpoint<E: MappingEndpoint>() {}

#[doc(hidden)]
pub const fn assert_bounded_context_endpoint<BC: BoundedContext>() {}
