use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::container_placement::command::AddContainer;
use crate::generated::domain::container_placement::error::ContainerPlacementError;
use crate::generated::domain::container_placement::event::*;
use chromatofoor::bounded_context::Command;

impl Command<ContainerPlacement> for AddContainer {
    type Error = ContainerPlacementError;

    #[tracing::instrument(skip(self, state), err)]
    async fn decide(
        self,
        state: &ContainerPlacement,
    ) -> Result<Vec<ContainerPlacementEvent>, Self::Error> {
        let Some(&room_max_capacity) = state.rooms.get(&self.room_id) else {
            return Err(ContainerPlacementError::RoomMustExist);
        };

        if state.room_occupancy(&self.room_id) >= room_max_capacity {
            return Err(ContainerPlacementError::RoomCapacityNotExceeded);
        }

        Ok(vec![
            ContainerAdded {
                container_id: self.container_id,
                room_id: self.room_id,
                container_type: self.container_type,
                max_capacity: self.max_capacity,
                apartment_id: self.apartment_id,
            }
            .into(),
        ])
    }
}
