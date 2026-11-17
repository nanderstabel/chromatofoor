use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::event::FlightReserved;
use chromatofoor::bounded_context::Event;
impl Event<FlightReservation> for FlightReserved {
    fn evolve(&self, _state: FlightReservation) -> FlightReservation {
        todo!(
            "implement Event::FlightReservation::evolve for FlightReserved (return _state if stateless)"
        )
    }
}
