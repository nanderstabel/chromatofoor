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
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &BookingPayment) -> Result<Vec<BookingPaymentEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![BookingPaymentEvent::PaymentFailed(PaymentFailed {
            booking_id: self.booking_id,
            payment_id: self.payment_id,
            reason: self.reason,
        })])
    }
}
