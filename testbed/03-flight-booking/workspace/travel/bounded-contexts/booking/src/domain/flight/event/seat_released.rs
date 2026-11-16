use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::event::SeatReleased;
use chromatofoor::bounded_context::Event;

impl Event<Flight> for SeatReleased {
    fn evolve(&self, mut state: Flight) -> Flight {
        if let Flight::Created(inner) = &mut state {
            inner.reserved_seats -= 1;
        }
        state
    }
}
