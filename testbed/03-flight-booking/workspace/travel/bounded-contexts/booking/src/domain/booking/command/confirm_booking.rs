use crate::generated::domain::booking::aggregate::Booking;
use crate::generated::domain::booking::command::ConfirmBooking;
use crate::generated::domain::booking::error::BookingError;
use crate::generated::domain::booking::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Booking> for ConfirmBooking {
    type Error = BookingError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Booking) -> Result<Vec<BookingEvent>, Self::Error> {
        let booking_id = match _aggregate {
            Booking::Created(inner) => {
                if inner.status != "Initialized" {
                    return Err(BookingError::Validation(format!(
                        "Cannot confirm booking because it is in state: {}",
                        inner.status
                    )));
                }
                inner.booking_id.clone()
            }
            Booking::None => {
                return Err(BookingError::Validation(
                    "Cannot confirm a non-existent booking".to_string(),
                ));
            }
        };

        Ok(vec![BookingConfirmed { booking_id }.into()])
    }
}
