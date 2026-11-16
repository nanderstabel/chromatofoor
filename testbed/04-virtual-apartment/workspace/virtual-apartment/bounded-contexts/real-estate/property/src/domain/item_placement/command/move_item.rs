use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use crate::generated::domain::item_placement::command::MoveItem;
use crate::generated::domain::item_placement::command::move_item::MoveItemTargetType;
use crate::generated::domain::item_placement::error::ItemPlacementError;
use crate::generated::domain::item_placement::event::ItemPlacementEvent;
use crate::generated::domain::item_placement::event::item_moved::{ItemMoved, ItemMovedTargetType};
use chromatofoor::bounded_context::Command;

impl Command<ItemPlacement> for MoveItem {
    type Error = ItemPlacementError;

    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &ItemPlacement,
    ) -> Result<Vec<ItemPlacementEvent>, Self::Error> {
        let target_type_str = match self.target_type {
            MoveItemTargetType::Room => "room",
            MoveItemTargetType::Container => "container",
        };

        let state = aggregate
            .fetch_target_state(
                &self.apartment_id,
                &self.target_id,
                target_type_str,
                &self.item_id,
            )
            .map_err(|_| ItemPlacementError::Validation("Failed to fetch state".into()))?;

        if !state.exists {
            return Err(ItemPlacementError::TargetMustExist);
        }

        if state.is_item_removed {
            return Err(ItemPlacementError::ItemNotRemoved);
        }

        if state.current_items_count >= state.max_capacity {
            return Err(ItemPlacementError::TargetCapacityNotExceeded);
        }

        let event_target_type = match self.target_type {
            MoveItemTargetType::Room => ItemMovedTargetType::Room,
            MoveItemTargetType::Container => ItemMovedTargetType::Container,
        };

        Ok(vec![ItemPlacementEvent::ItemMoved(ItemMoved {
            apartment_id: self.apartment_id,
            item_id: self.item_id,
            target_id: self.target_id,
            target_type: event_target_type,
        })])
    }
}
