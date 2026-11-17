use crate::generated::domain::shopping_cart::{
    aggregate::ShoppingCart, command::InitializeCart, error::ShoppingCartError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `initialize-cart` — Initializes a new empty shopping cart for a customer.
///
/// - Actors: `customer`
/// - Publishes: `cart-initialized`
/// - Invariants (`ShoppingCartError`):
///   - `empty-cart` — Cannot check out an empty shopping cart.
///   - `cart-already-checked-out` — Cannot modify or re-submit a shopping cart that has already been checked out.
impl Command<ShoppingCart> for InitializeCart {
    type Error = ShoppingCartError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &ShoppingCart) -> Result<Vec<ShoppingCartEvent>, Self::Error> {
        Ok(vec![ShoppingCartEvent::CartInitialized(CartInitialized {
            customer_id: self.customer_id,
        })])
    }
}
