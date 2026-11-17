use crate::generated::domain::shopping_cart::{
    aggregate::{ShoppingCart, ShoppingCartInner},
    event::CartInitialized,
};
use chromatofoor::bounded_context::Event;
/// Event: `cart-initialized` — Emitted when a new shopping cart is initialized.
impl Event<ShoppingCart> for CartInitialized {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        ShoppingCart::Created(ShoppingCartInner {
            checked_out: false,
            customer_id: self.customer_id.clone(),
            items: Vec::new(),
        })
    }
}
