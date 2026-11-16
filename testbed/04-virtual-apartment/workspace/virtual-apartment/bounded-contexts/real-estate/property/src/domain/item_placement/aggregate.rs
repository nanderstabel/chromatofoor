use crate::generated::domain::item_placement::aggregate::ItemPlacement;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub struct TargetState {
    pub exists: bool,
    pub max_capacity: i64,
    pub current_items_count: i64,
    pub is_item_removed: bool,
}

#[derive(Default, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ItemPlacementState {
    pub rooms: BTreeMap<uuid::Uuid, i64>,
    pub containers: BTreeMap<uuid::Uuid, i64>,
    pub container_locations: BTreeMap<uuid::Uuid, uuid::Uuid>,
    pub item_locations: BTreeMap<uuid::Uuid, uuid::Uuid>,
    pub removed_items: BTreeSet<uuid::Uuid>,
}

impl ItemPlacement {
    pub fn fetch_target_state(
        &self,
        _apartment_id: &uuid::Uuid,
        target_id: &uuid::Uuid,
        target_type: &str,
        item_id: &uuid::Uuid,
    ) -> Result<TargetState, anyhow::Error> {
        let (exists, max_capacity) = match target_type {
            "room" => match self.rooms.get(target_id) {
                Some(&cap) => (true, cap),
                None => (false, 0),
            },
            "container" => match self.containers.get(target_id) {
                Some(&cap) => (true, cap),
                None => (false, 0),
            },
            _ => (false, 0),
        };

        let is_item_removed = self.removed_items.contains(item_id);

        let mut current_items_count = self
            .item_locations
            .values()
            .filter(|&loc| loc == target_id)
            .count() as i64;

        if target_type == "room" {
            current_items_count += self
                .container_locations
                .values()
                .filter(|&loc| loc == target_id)
                .count() as i64;
        }

        Ok(TargetState {
            exists,
            max_capacity,
            current_items_count,
            is_item_removed,
        })
    }
}
