use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::ItemAddedToCart;
use chromatofoor::bounded_context::Event;
impl Event<ShoppingCart> for ItemAddedToCart {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for ItemAddedToCart (return _state if stateless)"
        )
    }
}
