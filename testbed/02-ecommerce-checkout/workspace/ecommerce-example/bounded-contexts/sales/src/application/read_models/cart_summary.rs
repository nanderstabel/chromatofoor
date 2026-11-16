use crate::generated::application::read_models::cart_summary::CartSummary;
use crate::generated::domain::shopping_cart::event::{
    cart_checked_out::CartCheckedOut, cart_initialized::CartInitialized,
    item_added_to_cart::ItemAddedToCart,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<CartInitialized> for CartSummary {
    async fn project(&self, event: Envelope<CartInitialized>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.status = Some("Open".to_string());
            rec.item_count = Some(0);
            rec.total_amount = Some(0);
        })?;
        Ok(())
    }
}

impl Project<ItemAddedToCart> for CartSummary {
    async fn project(&self, event: Envelope<ItemAddedToCart>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            let current_items = rec.item_count.unwrap_or(0);
            let current_total = rec.total_amount.unwrap_or(0);
            rec.item_count = Some(current_items + event.data.quantity);
            rec.total_amount = Some(current_total + (event.data.price * event.data.quantity));
        })?;
        Ok(())
    }
}

impl Project<CartCheckedOut> for CartSummary {
    async fn project(&self, event: Envelope<CartCheckedOut>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.status = Some("CheckedOut".to_string());
        })?;
        Ok(())
    }
}
