use crate::generated::domain::shopping_cart::{
    aggregate::ShoppingCart, command::AddItemToCart, error::ShoppingCartError, event::*,
};
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
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        let ShoppingCart::Created(inner) = state else {
            return Ok(vec![]);
        };
        if inner.checked_out {
            return Err(ShoppingCartError::CartAlreadyCheckedOut);
        }
        Ok(vec![ShoppingCartEvent::ItemAddedToCart(ItemAddedToCart {
            item: self.item,
        })])
    }
}
