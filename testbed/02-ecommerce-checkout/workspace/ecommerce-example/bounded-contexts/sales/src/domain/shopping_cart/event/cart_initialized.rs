use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::CartInitialized;
use chromatofoor::bounded_context::Event;
impl Event<ShoppingCart> for CartInitialized {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for CartInitialized (return _state if stateless)"
        )
    }
}
