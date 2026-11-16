use crate::generated::domain::booking::aggregate::Booking;
use crate::generated::domain::booking::event::BookingConfirmed;
use chromatofoor::bounded_context::Event;

impl Event<Booking> for BookingConfirmed {
    fn evolve(&self, mut state: Booking) -> Booking {
        if let Booking::Created(inner) = &mut state {
            inner.status = "Confirmed".to_string();
        }
        state
    }
}
