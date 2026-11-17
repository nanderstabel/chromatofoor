use crate::generated::domain::shopping_cart::{
    aggregate::ShoppingCart, command::CheckoutCart, error::ShoppingCartError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `checkout-cart` — Checks out a non-empty open cart with a validated shipping address.
///
/// - Actors: `customer`
/// - Publishes: `cart-checked-out`
/// - Invariants (`ShoppingCartError`):
///   - `empty-cart` — Cannot check out an empty shopping cart.
///   - `cart-already-checked-out` — Cannot modify or re-submit a shopping cart that has already been checked out.
impl Command<ShoppingCart> for CheckoutCart {
    type Error = ShoppingCartError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        todo!("implement Command::ShoppingCart::decide for CheckoutCart")
    }
}
