use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::ItemAddedToCart;
use chromatofoor::bounded_context::Event;
/// Event: `item-added-to-cart` — Emitted when a product line item is added to an open shopping cart.
impl Event<ShoppingCart> for ItemAddedToCart {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for ItemAddedToCart (return _state if stateless)"
        )
    }
}
