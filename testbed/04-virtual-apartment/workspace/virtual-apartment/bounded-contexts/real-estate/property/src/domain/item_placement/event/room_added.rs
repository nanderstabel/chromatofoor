use crate::generated::domain::apartment::event::room_added::RoomAdded;
use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for RoomAdded {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state.rooms.insert(self.room_id, self.max_capacity);
        state
    }
}
