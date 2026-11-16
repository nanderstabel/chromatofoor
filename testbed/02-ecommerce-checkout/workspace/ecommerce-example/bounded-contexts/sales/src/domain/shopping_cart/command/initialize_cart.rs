use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::command::InitializeCart;
use crate::generated::domain::shopping_cart::event::*;
use chromatofoor::bounded_context::Command;

impl Command<ShoppingCart> for InitializeCart {
    type Error = crate::generated::domain::shopping_cart::error::ShoppingCartError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        if !aggregate.is_none() {
            return Err(
                crate::generated::domain::shopping_cart::error::ShoppingCartError::Validation(
                    "Cart already initialized".to_string(),
                ),
            );
        }
        Ok(vec![
            crate::generated::domain::shopping_cart::event::ShoppingCartEvent::CartInitialized(
                crate::generated::domain::shopping_cart::event::cart_initialized::CartInitialized {
                    customer_id: self.customer_id,
                },
            ),
        ])
    }
}
