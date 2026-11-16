use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use crate::generated::domain::item_placement::event::ItemPlaced;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for ItemPlaced {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state.item_locations.insert(self.item_id, self.target_id);
        state
    }
}
