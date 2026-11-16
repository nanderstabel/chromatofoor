use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::container_placement::event::ContainerRemoved;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for ContainerRemoved {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state.container_locations.remove(&self.container_id);
        state
    }
}
