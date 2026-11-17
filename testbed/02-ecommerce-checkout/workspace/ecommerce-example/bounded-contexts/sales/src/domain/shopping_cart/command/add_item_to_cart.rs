use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::command::AddItemToCart;
use crate::generated::domain::shopping_cart::error::ShoppingCartError;
use crate::generated::domain::shopping_cart::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `add-item-to-cart` — Adds a validated product line item to an open shopping cart.
///
/// - Actors: `customer`
/// - Publishes: `item-added-to-cart`
/// - Invariants (`ShoppingCartError`):
///   - `empty-cart` — Cannot check out an empty shopping cart.
///   - `cart-already-checked-out` — Cannot modify or re-submit a shopping cart that has already been checked out.
impl Command<ShoppingCart> for AddItemToCart {
    type Error = ShoppingCartError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        todo!("implement Command::ShoppingCart::decide for AddItemToCart")
    }
}
