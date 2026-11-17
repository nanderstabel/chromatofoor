use crate::generated::domain::booking_payment::aggregate::BookingPayment;
use crate::generated::domain::booking_payment::command::RequestPayment;
use crate::generated::domain::booking_payment::error::BookingPaymentError;
use crate::generated::domain::booking_payment::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `request-payment` — Initiates a payment request once flight and hotel holds are in place.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `payment-requested`
impl Command<BookingPayment> for RequestPayment {
    type Error = BookingPaymentError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &BookingPayment,
    ) -> Result<Vec<BookingPaymentEvent>, Self::Error> {
        todo!("implement Command::BookingPayment::decide for RequestPayment")
    }
}
