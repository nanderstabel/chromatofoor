use crate::generated::domain::cargo_booking::{
    aggregate::{CargoBooking, CargoBookingInner},
    event::CargoBooked,
};
use chromatofoor::bounded_context::Event;
/// Event: `cargo-booked` — Emitted when a new cargo shipment is booked.
impl Event<CargoBooking> for CargoBooked {
    fn evolve(&self, _state: CargoBooking) -> CargoBooking {
        CargoBooking::Created(CargoBookingInner {
            assigned_voyage: None,
            destination_locode: self.destination_locode.clone(),
            origin_locode: self.origin_locode.clone(),
            tracking_id: self.tracking_id.clone(),
        })
    }
}
