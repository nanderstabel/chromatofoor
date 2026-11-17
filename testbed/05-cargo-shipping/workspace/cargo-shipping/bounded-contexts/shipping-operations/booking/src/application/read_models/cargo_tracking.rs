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
/// Rule: Insert cargo-tracking record keyed by tracking-id with routing-status NotRouted.
impl Project<CargoBooked> for CargoTracking {
    async fn project(&self, event: Envelope<CargoBooked>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.tracking_id.as_str(), &event, |record| {
                record.tracking_id = event.data.tracking_id.to_string();
                record.origin_locode.clone_from(&event.data.origin_locode);
                record
                    .destination_locode
                    .clone_from(&event.data.destination_locode);
                record.routing_status = "NotRouted".to_owned();
                record.voyage_number = None;
            })?;
        Ok(())
    }
}
/// Read Model: `cargo-tracking` — Materialized shipment tracking view showing route ports and assigned voyage.
/// Projection: `cargo-assigned-to-route`
///
/// Rule: Update routing-status to Routed and set voyage-number for the record keyed by tracking-id.
impl Project<CargoAssignedToRoute> for CargoTracking {
    async fn project(&self, event: Envelope<CargoAssignedToRoute>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.tracking_id.as_str(), &event, |record| {
                record.tracking_id = event.data.tracking_id.to_string();
                record.routing_status = "Routed".to_owned();
                record.voyage_number = Some(event.data.voyage_number.clone());
            })?;
        Ok(())
    }
}
