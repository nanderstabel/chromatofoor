use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::CartCheckedOut;
use chromatofoor::bounded_context::Event;
impl Event<ShoppingCart> for CartCheckedOut {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        todo!(
            "implement Event::ShoppingCart::evolve for CartCheckedOut (return _state if stateless)"
        )
    }
}
