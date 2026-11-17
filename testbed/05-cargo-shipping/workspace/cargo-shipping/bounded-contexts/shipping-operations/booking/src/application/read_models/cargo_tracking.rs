#[allow(unused_imports)]
use crate::generated::application::read_models::cargo_tracking::{
    CargoTracking, CargoTrackingRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::cargo_booking::event::cargo_booked::CargoBooked>
    for CargoTracking
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::cargo_booking::event::cargo_booked::CargoBooked>,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CargoBooked::project for CargoTracking")
    }
}
impl Project<
    crate::generated::domain::cargo_booking::event::cargo_assigned_to_route::CargoAssignedToRoute,
> for CargoTracking {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::cargo_booking::event::cargo_assigned_to_route::CargoAssignedToRoute,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::CargoAssignedToRoute::project for CargoTracking")
    }
}
