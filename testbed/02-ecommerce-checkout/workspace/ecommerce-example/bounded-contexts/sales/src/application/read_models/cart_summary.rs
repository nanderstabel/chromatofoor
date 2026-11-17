#[allow(unused_imports)]
use crate::generated::application::read_models::cart_summary::{CartSummary, CartSummaryRecord};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `cart-initialized`
///
/// Rule: Initialize total-amount to 0, item-count to 0, and status to Open.
impl Project<crate::generated::domain::shopping_cart::event::cart_initialized::CartInitialized>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::cart_initialized::CartInitialized,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CartInitialized::project for CartSummary")
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `item-added-to-cart`
///
/// Rule: Add item.quantity * item.unit-price to total-amount and add item.quantity to item-count.
impl Project<crate::generated::domain::shopping_cart::event::item_added_to_cart::ItemAddedToCart>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::item_added_to_cart::ItemAddedToCart,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::ItemAddedToCart::project for CartSummary")
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `cart-checked-out`
///
/// Rule: Set status to CheckedOut.
impl Project<crate::generated::domain::shopping_cart::event::cart_checked_out::CartCheckedOut>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::cart_checked_out::CartCheckedOut,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CartCheckedOut::project for CartSummary")
    }
}
