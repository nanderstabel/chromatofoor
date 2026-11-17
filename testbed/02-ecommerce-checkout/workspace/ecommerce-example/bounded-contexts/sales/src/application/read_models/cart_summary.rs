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
/// Rule: Initialize record keyed by event subject ID with total-amount 0, item-count 0, and status Open.
impl Project<CartInitialized> for CartSummary {
    async fn project(&self, event: Envelope<CartInitialized>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.total_amount = 0;
            record.item_count = 0;
            record.status = "Open".to_owned();
        })?;
        Ok(())
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `item-added-to-cart`
///
/// Rule: Add item.quantity * item.unit-price to total-amount and add item.quantity to item-count for the event subject ID.
impl Project<ItemAddedToCart> for CartSummary {
    async fn project(&self, event: Envelope<ItemAddedToCart>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.total_amount += event.data.item.quantity * event.data.item.unit_price;
            record.item_count += event.data.item.quantity;
        })?;
        Ok(())
    }
}
/// Read Model: `cart-summary` — Materialized shopping cart summary displaying item count, total amount, and checkout status.
/// Projection: `cart-checked-out`
///
/// Rule: Set status to CheckedOut for the event subject ID.
impl Project<CartCheckedOut> for CartSummary {
    async fn project(&self, event: Envelope<CartCheckedOut>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.status = "CheckedOut".to_owned();
        })?;
        Ok(())
    }
}
