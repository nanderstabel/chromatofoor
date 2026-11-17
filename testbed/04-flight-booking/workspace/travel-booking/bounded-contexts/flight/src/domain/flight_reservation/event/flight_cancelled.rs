use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, event::FlightCancelled,
};
use chromatofoor::bounded_context::Event;
/// Event: `flight-cancelled` — Emitted when a held flight reservation is cancelled.
impl Event<FlightReservation> for FlightCancelled {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        todo!(
            "implement Event::FlightReservation::evolve for FlightCancelled (return _state if stateless)"
        )
    }
}
