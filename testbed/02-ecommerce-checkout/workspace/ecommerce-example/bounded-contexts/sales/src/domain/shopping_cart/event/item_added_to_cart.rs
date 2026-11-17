use crate::generated::domain::shopping_cart::{aggregate::ShoppingCart, event::ItemAddedToCart};
use chromatofoor::bounded_context::Event;
/// Event: `item-added-to-cart` — Emitted when a product line item is added to an open shopping cart.
impl Event<ShoppingCart> for ItemAddedToCart {
    fn evolve(&self, mut state: ShoppingCart) -> ShoppingCart {
        if let ShoppingCart::Created(ref mut inner) = state {
            inner.items.push(self.item.clone());
        }
        state
    }
}
