#[allow(unused_imports)]
use crate::generated::application::read_models::apartment_inventory::{
    ApartmentInventory, ApartmentInventoryRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::apartment::event::registered::Registered>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::apartment::event::registered::Registered>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::apartment::event::unregistered::Unregistered>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::apartment::event::unregistered::Unregistered>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::apartment::event::room_added::RoomAdded>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::apartment::event::room_added::RoomAdded>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::apartment::event::room_removed::RoomRemoved>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::apartment::event::room_removed::RoomRemoved>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::container_placement::event::container_added::ContainerAdded>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::container_placement::event::container_added::ContainerAdded,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl
    Project<
        crate::generated::domain::container_placement::event::container_removed::ContainerRemoved,
    > for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::container_placement::event::container_removed::ContainerRemoved,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::item_placement::event::item_placed::ItemPlaced>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::item_placement::event::item_placed::ItemPlaced>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::item_placement::event::item_moved::ItemMoved>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::item_placement::event::item_moved::ItemMoved>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::item_placement::event::item_removed::ItemRemoved>
    for ApartmentInventory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::item_placement::event::item_removed::ItemRemoved,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
