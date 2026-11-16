use crate::generated::domain::apartment::event::room_added::RoomAdded;
use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for RoomAdded {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state.rooms.insert(self.room_id, self.max_capacity);
        state
    }
}
