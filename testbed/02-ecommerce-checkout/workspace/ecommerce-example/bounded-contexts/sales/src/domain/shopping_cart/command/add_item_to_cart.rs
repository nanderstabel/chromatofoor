use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::command::AddItemToCart;
use crate::generated::domain::shopping_cart::event::*;
use chromatofoor::bounded_context::Command;

impl Command<ShoppingCart> for AddItemToCart {
    type Error = crate::generated::domain::shopping_cart::error::ShoppingCartError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        match aggregate {
            ShoppingCart::Created(inner) => {
                if inner.status.as_deref() == Some("CheckedOut") {
                    return Err(crate::generated::domain::shopping_cart::error::ShoppingCartError::Validation(
                        "Cannot add items to a checked out cart".to_string()
                    ));
                }
                Ok(vec![crate::generated::domain::shopping_cart::event::ShoppingCartEvent::ItemAddedToCart(
                        crate::generated::domain::shopping_cart::event::item_added_to_cart::ItemAddedToCart {
                            product_id: self.product_id,
                            quantity: self.quantity,
                            price: self.price,
                        }
                    )])
            }
            ShoppingCart::None => Err(
                crate::generated::domain::shopping_cart::error::ShoppingCartError::Validation(
                    "Cart not found".to_string(),
                ),
            ),
        }
    }
}
