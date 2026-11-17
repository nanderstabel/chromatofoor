use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::command::AddItemToCart;
use crate::generated::domain::shopping_cart::error::ShoppingCartError;
use crate::generated::domain::shopping_cart::event::*;
use chromatofoor::bounded_context::Command;
impl Command<ShoppingCart> for AddItemToCart {
    type Error = ShoppingCartError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        todo!("implement Command::ShoppingCart::decide for AddItemToCart")
    }
}
