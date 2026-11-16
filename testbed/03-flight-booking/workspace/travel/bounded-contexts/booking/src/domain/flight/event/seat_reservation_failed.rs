use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::event::SeatReservationFailed;
use chromatofoor::bounded_context::Event;

impl Event<Flight> for SeatReservationFailed {
    fn evolve(&self, state: Flight) -> Flight {
        // No state change needed for failed reservation
        state
    }
}
