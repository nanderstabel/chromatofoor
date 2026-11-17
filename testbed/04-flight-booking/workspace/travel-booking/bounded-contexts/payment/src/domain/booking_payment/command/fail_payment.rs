use crate::generated::domain::booking_payment::{
    aggregate::BookingPayment, command::FailPayment, error::BookingPaymentError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `fail-payment` — Records a declined payment attempt.
///
/// - Actors: `traveler`, `booking-coordinator`
/// - Publishes: `payment-failed`
impl Command<BookingPayment> for FailPayment {
    type Error = BookingPaymentError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &BookingPayment,
    ) -> Result<Vec<BookingPaymentEvent>, Self::Error> {
        todo!("implement Command::BookingPayment::decide for FailPayment")
    }
}
