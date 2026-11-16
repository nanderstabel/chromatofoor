use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::container_placement::event::ContainerAdded;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for ContainerAdded {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state
            .container_locations
            .insert(self.container_id, self.room_id);
        state
    }
}
