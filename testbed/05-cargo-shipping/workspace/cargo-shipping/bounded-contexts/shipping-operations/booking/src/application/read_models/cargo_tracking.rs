#[allow(unused_imports)]
use crate::generated::application::read_models::cargo_tracking::{
    CargoTracking, CargoTrackingRecord,
};
use crate::generated::domain::cargo_booking::event::CargoAssignedToRoute;
use crate::generated::domain::cargo_booking::event::CargoBooked;
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `cargo-tracking` — Materialized shipment tracking view showing route ports and assigned voyage.
/// Projection: `cargo-booked`
///
/// Rule: Insert cargo-tracking record with routing-status NotRouted.
impl Project<CargoBooked> for CargoTracking {
    async fn project(&self, _event: Envelope<CargoBooked>) -> Result<(), ViewError> {
        todo!("implement Project::CargoBooked::project for CargoTracking")
    }
}
/// Read Model: `cargo-tracking` — Materialized shipment tracking view showing route ports and assigned voyage.
/// Projection: `cargo-assigned-to-route`
///
/// Rule: Update routing-status to Routed and set voyage-number.
impl Project<CargoAssignedToRoute> for CargoTracking {
    async fn project(&self, _event: Envelope<CargoAssignedToRoute>) -> Result<(), ViewError> {
        todo!("implement Project::CargoAssignedToRoute::project for CargoTracking")
    }
}
