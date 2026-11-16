use crate::generated::domain::container_placement::event::container_added::ContainerAdded;
use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use chromatofoor::bounded_context::Event;

impl Event<ItemPlacement> for ContainerAdded {
    fn evolve(&self, mut state: ItemPlacement) -> ItemPlacement {
        state
            .containers
            .insert(self.container_id, self.max_capacity);
        state
            .container_locations
            .insert(self.container_id, self.room_id);
        state
    }
}
