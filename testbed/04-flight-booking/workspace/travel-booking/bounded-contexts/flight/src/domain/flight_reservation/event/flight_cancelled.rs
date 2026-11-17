use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, event::FlightCancelled,
};
use chromatofoor::bounded_context::Event;
/// Event: `flight-cancelled` — Emitted when a held flight reservation is cancelled.
impl Event<FlightReservation> for FlightCancelled {
    fn evolve(&self, mut state: FlightReservation) -> FlightReservation {
        if let FlightReservation::Created(ref mut inner) = state {
            inner.status = "Cancelled".to_owned();
        }
        state
    }
}
