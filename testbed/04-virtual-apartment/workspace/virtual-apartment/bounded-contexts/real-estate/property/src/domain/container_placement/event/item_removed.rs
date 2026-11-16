use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::item_placement::event::item_removed::ItemRemoved;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for ItemRemoved {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state.item_locations.remove(&self.item_id);
        state
    }
}
