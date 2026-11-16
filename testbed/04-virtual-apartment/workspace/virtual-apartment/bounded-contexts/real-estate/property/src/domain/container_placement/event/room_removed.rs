use crate::generated::domain::apartment::event::room_removed::RoomRemoved;
use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for RoomRemoved {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state.rooms.remove(&self.room_id);
        state
    }
}
