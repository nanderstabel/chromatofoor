use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::event::FlightConfirmed;
use chromatofoor::bounded_context::Event;
/// Event: `flight-confirmed` — Emitted when a held flight reservation is confirmed.
impl Event<FlightReservation> for FlightConfirmed {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        todo!(
            "implement Event::FlightReservation::evolve for FlightConfirmed (return _state if stateless)"
        )
    }
}
