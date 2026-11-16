use crate::generated::domain::booking::aggregate::Booking;
use crate::generated::domain::booking::event::BookingFailed;
use chromatofoor::bounded_context::Event;

impl Event<Booking> for BookingFailed {
    fn evolve(&self, mut state: Booking) -> Booking {
        if let Booking::Created(inner) = &mut state {
            inner.status = "Failed".to_string();
        }
        state
    }
}
