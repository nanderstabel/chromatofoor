use crate::generated::domain::booking_payment::{
    aggregate::BookingPayment, command::CompletePayment, error::BookingPaymentError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `complete-payment` — Records successful payment capture by the traveler.
///
/// - Actors: `traveler`
/// - Publishes: `payment-completed`
impl Command<BookingPayment> for CompletePayment {
    type Error = BookingPaymentError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &BookingPayment) -> Result<Vec<BookingPaymentEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![BookingPaymentEvent::PaymentCompleted(
            PaymentCompleted {
                booking_id: self.booking_id,
                payment_id: self.payment_id,
            },
        )])
    }
}
