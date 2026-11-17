#[allow(unused_imports)]
use crate::generated::application::read_models::cart_summary::{CartSummary, CartSummaryRecord};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::shopping_cart::event::cart_initialized::CartInitialized>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::cart_initialized::CartInitialized,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CartInitialized::project for CartSummary")
    }
}
impl Project<crate::generated::domain::shopping_cart::event::item_added_to_cart::ItemAddedToCart>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::item_added_to_cart::ItemAddedToCart,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::ItemAddedToCart::project for CartSummary")
    }
}
impl Project<crate::generated::domain::shopping_cart::event::cart_checked_out::CartCheckedOut>
    for CartSummary
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::shopping_cart::event::cart_checked_out::CartCheckedOut,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CartCheckedOut::project for CartSummary")
    }
}
