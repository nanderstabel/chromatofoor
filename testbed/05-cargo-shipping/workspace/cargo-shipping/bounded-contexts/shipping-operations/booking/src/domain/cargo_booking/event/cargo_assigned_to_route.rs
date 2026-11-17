use crate::generated::domain::cargo_booking::{
    aggregate::CargoBooking, event::CargoAssignedToRoute,
};
use chromatofoor::bounded_context::Event;
/// Event: `cargo-assigned-to-route` — Emitted when a booked cargo shipment is assigned to a vessel voyage.
impl Event<CargoBooking> for CargoAssignedToRoute {
    fn evolve(&self, mut state: CargoBooking) -> CargoBooking {
        if let CargoBooking::Created(ref mut inner) = state {
            inner.assigned_voyage = Some(self.voyage_number.clone());
        }
        state
    }
}
