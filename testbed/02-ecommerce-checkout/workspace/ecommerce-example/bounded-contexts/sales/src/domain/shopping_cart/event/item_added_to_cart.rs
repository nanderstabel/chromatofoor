use crate::generated::domain::shopping_cart::aggregate::ShoppingCart;
use crate::generated::domain::shopping_cart::event::ItemAddedToCart;
use chromatofoor::bounded_context::Event;

impl Event<ShoppingCart> for ItemAddedToCart {
    fn evolve(&self, mut state: ShoppingCart) -> ShoppingCart {
        if let ShoppingCart::Created(inner) = &mut state {
            if let Some(items) = inner.items.as_mut() {
                items.push(
                    crate::generated::domain::shopping_cart::aggregate::ShoppingCartItemsItem {
                        product_id: self.product_id.clone(),
                        quantity: self.quantity,
                        price: self.price,
                    },
                );
            }
        }
        state
    }
}
