use crate::generated::domain::shopping_cart::{aggregate::ShoppingCart, event::CartCheckedOut};
use chromatofoor::bounded_context::Event;
/// Event: `cart-checked-out` — Emitted when a shopping cart is checked out with a shipping address.
impl Event<ShoppingCart> for CartCheckedOut {
    fn evolve(&self, mut state: ShoppingCart) -> ShoppingCart {
        if let ShoppingCart::Created(ref mut inner) = state {
            inner.checked_out = true;
        }
        state
    }
}
