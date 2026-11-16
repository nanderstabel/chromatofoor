use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::item_placement::event::item_placed::ItemPlaced;
use chromatofoor::bounded_context::Event;

impl Event<ContainerPlacement> for ItemPlaced {
    fn evolve(&self, mut state: ContainerPlacement) -> ContainerPlacement {
        state.item_locations.insert(self.item_id, self.target_id);
        state
    }
}
