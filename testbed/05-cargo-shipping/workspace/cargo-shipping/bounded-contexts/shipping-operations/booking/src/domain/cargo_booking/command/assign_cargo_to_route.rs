use crate::generated::domain::cargo_booking::{
    aggregate::CargoBooking, command::AssignCargoToRoute, error::CargoBookingError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `assign-cargo-to-route` — Assigns an existing cargo booking to a scheduled vessel voyage.
///
/// - Actors: `shipper`
/// - Publishes: `cargo-assigned-to-route`
/// - Invariants (`CargoBookingError`):
///   - `same-origin-and-destination` — Origin UN/LOCODE and destination UN/LOCODE cannot be identical.
impl Command<CargoBooking> for AssignCargoToRoute {
    type Error = CargoBookingError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &CargoBooking) -> Result<Vec<CargoBookingEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![CargoBookingEvent::CargoAssignedToRoute(
            CargoAssignedToRoute {
                tracking_id: self.tracking_id,
                voyage_number: self.voyage_number,
            },
        )])
    }
}
