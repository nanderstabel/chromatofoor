use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use crate::generated::domain::item_placement::command::RemoveItem;
use crate::generated::domain::item_placement::error::ItemPlacementError;
use crate::generated::domain::item_placement::event::ItemPlacementEvent;
use crate::generated::domain::item_placement::event::item_removed::ItemRemoved;
use chromatofoor::bounded_context::Command;

impl Command<ItemPlacement> for RemoveItem {
    type Error = ItemPlacementError;

    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &ItemPlacement,
    ) -> Result<Vec<ItemPlacementEvent>, Self::Error> {
        let state = aggregate
            .fetch_target_state(&self.apartment_id, &self.target_id, "room", &self.item_id)
            .map_err(|_| ItemPlacementError::Validation("Failed to fetch state".into()))?;

        if state.is_item_removed {
            return Err(ItemPlacementError::ItemNotRemoved);
        }

        Ok(vec![ItemPlacementEvent::ItemRemoved(ItemRemoved {
            apartment_id: self.apartment_id,
            item_id: self.item_id,
            target_id: self.target_id,
        })])
    }
}
