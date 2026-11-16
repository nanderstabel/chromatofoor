use crate::{
    bounded_context::BoundedContext,
    common::{Identified, Kind},
};

/// An ESDM `entity` — an identity-bearing modeling element within a [`BoundedContext`]
/// that has no consistency container of its own (`LineItem`, `ReturnItem`).
///
/// [`#[chromatofoor::entity(scope = ...)]`](crate::entity) derives `Clone`, `Debug`,
/// `PartialEq`, `serde::Serialize`, `serde::Deserialize`, and [`architecturekit::Schema`],
/// and implements [`Kind`], [`Identified`], and [`Entity`].
///
/// # Identification Strategies (`identified_by`)
///
/// ## 1. `schema(<field>)` *(default: `schema(id)`)*
///
/// Identified by an [`Identifier`](crate::common::Identifier) field within the entity's struct schema:
///
/// ```
/// # use chromatofoor::common::{IdSource, Identified, Identifier, Kind};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::entity(scope = Ordering, identified_by = schema(line_item_id))]
/// pub struct LineItem {
///     pub line_item_id: Identifier,
///     pub sku: String,
///     pub quantity: u32,
/// }
///
/// let item = LineItem {
///     line_item_id: "li-1".into(),
///     sku: "SKU-42".to_owned(),
///     quantity: 2,
/// };
/// assert_eq!(item.topic().as_str(), "/commerce/ordering/line-item");
/// assert_eq!(LineItem::IDENTIFIED_BY, &[IdSource::Schema("line_item_id")]);
/// ```
///
/// Referencing a nonexistent field in `identified_by = schema(...)` is rejected at
/// compile time:
///
/// ```compile_fail
/// # use chromatofoor::common::Identifier;
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::entity(scope = Ordering, identified_by = schema(missing_field))]
/// pub struct LineItem {
///     pub line_item_id: Identifier,
/// }
/// ```
///
/// ## 2. `static("<value>")`
///
/// Identified by a fixed identifier string:
///
/// ```
/// # use chromatofoor::common::{IdSource, Identified};
/// # #[chromatofoor::domain]
/// # pub struct Commerce;
/// # #[chromatofoor::bounded_context(scope = Commerce)]
/// # pub struct Ordering;
/// #[chromatofoor::entity(scope = Ordering, identified_by = static("default-cart"))]
/// pub struct DefaultCartConfig {
///     pub max_items: u32,
/// }
///
/// assert_eq!(DefaultCartConfig::IDENTIFIED_BY, &[IdSource::Static("default-cart")]);
/// ```
pub trait Entity: Kind<Scope: BoundedContext> + Identified + architecturekit::Schema {}
