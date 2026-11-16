use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::CartCheckedOut;
use chromatofoor::bounded_context::Event;

impl Event<ShoppingCart> for CartCheckedOut {
    fn evolve(&self, mut state: ShoppingCart) -> ShoppingCart {
        if let ShoppingCart::Created(inner) = &mut state {
            inner.status = Some("CheckedOut".to_string());
        }
        state
    }
}
