use crate::generated::domain::cargo_booking::aggregate::CargoBooking;
use crate::generated::domain::cargo_booking::event::CargoAssignedToRoute;
use chromatofoor::bounded_context::Event;
/// Event: `cargo-assigned-to-route` — Emitted when a booked cargo shipment is assigned to a vessel voyage.
impl Event<CargoBooking> for CargoAssignedToRoute {
    fn evolve(&self, _state: CargoBooking) -> CargoBooking {
        todo!(
            "implement Event::CargoBooking::evolve for CargoAssignedToRoute (return _state if stateless)"
        )
    }
}
