use crate::generated::domain::container_placement::aggregate::ContainerPlacement;
use crate::generated::domain::container_placement::command::RemoveContainer;
use crate::generated::domain::container_placement::error::ContainerPlacementError;
use crate::generated::domain::container_placement::event::{
    ContainerPlacementEvent, ContainerRemoved,
};
use chromatofoor::bounded_context::Command;

impl Command<ContainerPlacement> for RemoveContainer {
    type Error = ContainerPlacementError;

    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &ContainerPlacement,
    ) -> Result<Vec<ContainerPlacementEvent>, Self::Error> {
        Ok(vec![ContainerPlacementEvent::ContainerRemoved(
            ContainerRemoved {
                apartment_id: self.apartment_id,
                container_id: self.container_id,
                room_id: self.room_id,
            },
        )])
    }
}
