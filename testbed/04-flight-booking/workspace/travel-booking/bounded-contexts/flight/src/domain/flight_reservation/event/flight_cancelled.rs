use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::event::FlightCancelled;
use chromatofoor::bounded_context::Event;
impl Event<FlightReservation> for FlightCancelled {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        todo!(
            "implement Event::FlightReservation::evolve for FlightCancelled (return _state if stateless)"
        )
    }
}
