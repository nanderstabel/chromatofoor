use crate::generated::domain::shopping_cart::{aggregate::ShoppingCart, event::CartInitialized};
use chromatofoor::bounded_context::Event;
/// Event: `cart-initialized` — Emitted when a new shopping cart is initialized.
impl Event<ShoppingCart> for CartInitialized {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for CartInitialized (return _state if stateless)"
        )
    }
}
