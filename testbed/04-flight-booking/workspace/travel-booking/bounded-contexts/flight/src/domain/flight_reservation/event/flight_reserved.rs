use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, event::FlightReserved,
};
use chromatofoor::bounded_context::Event;
/// Event: `flight-reserved` — Emitted when a flight reservation hold is created.
impl Event<FlightReservation> for FlightReserved {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        todo!(
            "implement Event::FlightReservation::evolve for FlightReserved (return _state if stateless)"
        )
    }
}
