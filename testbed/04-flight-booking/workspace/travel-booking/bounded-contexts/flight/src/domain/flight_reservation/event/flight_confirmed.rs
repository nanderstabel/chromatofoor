use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, event::FlightConfirmed,
};
use chromatofoor::bounded_context::Event;
/// Event: `flight-confirmed` — Emitted when a held flight reservation is confirmed.
impl Event<FlightReservation> for FlightConfirmed {
    fn evolve(&self, mut state: FlightReservation) -> FlightReservation {
        if let FlightReservation::Created(ref mut inner) = state {
            inner.status = "Confirmed".to_owned();
        }
        state
    }
}
