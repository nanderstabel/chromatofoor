use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ContainerPlacementState {
    pub rooms: BTreeMap<uuid::Uuid, i64>,
    pub container_locations: BTreeMap<uuid::Uuid, uuid::Uuid>,
    pub item_locations: BTreeMap<uuid::Uuid, uuid::Uuid>,
}

impl ContainerPlacement {
    #[must_use]
    pub fn room_occupancy(&self, room_id: &uuid::Uuid) -> i64 {
        let containers = self
            .container_locations
            .values()
            .filter(|&loc| loc == room_id)
            .count();
        let items = self
            .item_locations
            .values()
            .filter(|&loc| loc == room_id)
            .count();
        (containers + items) as i64
    }
}
