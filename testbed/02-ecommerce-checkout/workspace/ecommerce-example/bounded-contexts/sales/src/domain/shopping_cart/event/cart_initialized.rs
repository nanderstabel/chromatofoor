use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::CartInitialized;
use chromatofoor::bounded_context::Event;

impl Event<ShoppingCart> for CartInitialized {
    fn evolve(&self, _state: ShoppingCart) -> ShoppingCart {
        ShoppingCart::Created(
            crate::generated::domain::shopping_cart::aggregate::ShoppingCartInner {
                status: Some("Open".to_string()),
                items: Some(vec![]),
            },
        )
    }
}
