use crate::generated::domain::booking::aggregate::Booking;
use crate::generated::domain::booking::command::InitializeBooking;
use crate::generated::domain::booking::error::BookingError;
use crate::generated::domain::booking::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Booking> for InitializeBooking {
    type Error = BookingError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Booking) -> Result<Vec<BookingEvent>, Self::Error> {
        Ok(vec![
            BookingInitialized {
                booking_id: self.booking_id,
                flight_id: self.flight_id,
                amount: self.amount,
            }
            .into(),
        ])
    }
}
