#[allow(unused_imports)]
use crate::generated::application::read_models::cart_summary::{CartSummary, CartSummaryRecord};
use crate::generated::domain::shopping_cart::event::CartCheckedOut;
use crate::generated::domain::shopping_cart::event::CartInitialized;
use crate::generated::domain::shopping_cart::event::ItemAddedToCart;
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `cart-initialized`
///
/// Rule: Initialize total-amount to 0, item-count to 0, and status to Open.
impl Project<CartInitialized> for CartSummary {
    async fn project(&self, _event: Envelope<CartInitialized>) -> Result<(), ViewError> {
        todo!("implement Project::CartInitialized::project for CartSummary")
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `item-added-to-cart`
///
/// Rule: Add item.quantity * item.unit-price to total-amount and add item.quantity to item-count.
impl Project<ItemAddedToCart> for CartSummary {
    async fn project(&self, _event: Envelope<ItemAddedToCart>) -> Result<(), ViewError> {
        todo!("implement Project::ItemAddedToCart::project for CartSummary")
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `cart-checked-out`
///
/// Rule: Set status to CheckedOut.
impl Project<CartCheckedOut> for CartSummary {
    async fn project(&self, _event: Envelope<CartCheckedOut>) -> Result<(), ViewError> {
        todo!("implement Project::CartCheckedOut::project for CartSummary")
    }
}
