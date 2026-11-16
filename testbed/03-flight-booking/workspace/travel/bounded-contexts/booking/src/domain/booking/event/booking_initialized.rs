use crate::generated::domain::booking::aggregate::Booking;
use crate::generated::domain::booking::event::BookingInitialized;
use chromatofoor::bounded_context::Event;

impl Event<Booking> for BookingInitialized {
    fn evolve(&self, _state: Booking) -> Booking {
        Booking::Created(crate::generated::domain::booking::aggregate::BookingInner {
            amount: self.amount,
            booking_id: self.booking_id.clone(),
            flight_id: self.flight_id.clone(),
            status: "Initialized".to_string(),
        })
    }
}
