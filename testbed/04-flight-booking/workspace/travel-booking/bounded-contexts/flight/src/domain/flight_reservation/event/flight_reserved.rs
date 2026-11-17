use crate::generated::domain::flight_reservation::{
    aggregate::{FlightReservation, FlightReservationInner},
    event::FlightReserved,
};
use chromatofoor::bounded_context::Event;
/// Event: `flight-reserved` — Emitted when a flight reservation hold is created.
impl Event<FlightReservation> for FlightReserved {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        FlightReservation::Created(FlightReservationInner {
            booking_id: self.booking_id.clone(),
            flight_number: self.flight_number.clone(),
            reservation_id: self.reservation_id.clone(),
            status: "Reserved".to_owned(),
        })
    }
}
