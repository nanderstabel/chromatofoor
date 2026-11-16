use crate::generated::domain::apartment::event::room_removed::RoomRemoved;
use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for RoomRemoved {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state.rooms.remove(&self.room_id);
        state
    }
}
