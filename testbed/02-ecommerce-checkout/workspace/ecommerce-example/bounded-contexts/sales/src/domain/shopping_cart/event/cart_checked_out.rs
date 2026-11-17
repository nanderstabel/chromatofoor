use crate::generated::domain::shopping_cart::{aggregate::ShoppingCart, event::CartCheckedOut};
use chromatofoor::bounded_context::Event;
/// Event: `cart-checked-out` — Emitted when a shopping cart is checked out with a shipping address.
impl Event<ShoppingCart> for CartCheckedOut {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for CartCheckedOut (return _state if stateless)"
        )
    }
}
