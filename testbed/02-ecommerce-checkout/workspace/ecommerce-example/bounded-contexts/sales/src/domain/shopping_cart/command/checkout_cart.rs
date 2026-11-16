use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::command::CheckoutCart;
use crate::generated::domain::shopping_cart::event::*;
use chromatofoor::bounded_context::Command;

impl Command<ShoppingCart> for CheckoutCart {
    type Error = crate::generated::domain::shopping_cart::error::ShoppingCartError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        match aggregate {
            ShoppingCart::Created(inner) => {
                if inner.items.as_ref().map(|i| i.len()).unwrap_or(0) == 0 {
                    return Err(crate::generated::domain::shopping_cart::error::ShoppingCartError::Validation(
                        "Cannot checkout an empty cart".to_string()
                    ));
                }
                if inner.status.as_deref() == Some("CheckedOut") {
                    return Err(crate::generated::domain::shopping_cart::error::ShoppingCartError::Validation(
                        "Cart already checked out".to_string()
                    ));
                }
                Ok(vec![crate::generated::domain::shopping_cart::event::ShoppingCartEvent::CartCheckedOut(
                        crate::generated::domain::shopping_cart::event::cart_checked_out::CartCheckedOut {
                            payment_method: self.payment_method,
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
