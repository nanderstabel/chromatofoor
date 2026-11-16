use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use crate::generated::domain::item_placement::event::ItemRemoved;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for ItemRemoved {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state.item_locations.remove(&self.item_id);
        state.removed_items.insert(self.item_id);
        state
    }
}
