use crate::generated::domain::container_placement::event::container_removed::ContainerRemoved;
use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for ContainerRemoved {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state.containers.remove(&self.container_id);
        state.container_locations.remove(&self.container_id);
        state
    }
}
