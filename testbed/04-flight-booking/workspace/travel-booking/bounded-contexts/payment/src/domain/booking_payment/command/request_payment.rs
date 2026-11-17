use crate::generated::domain::booking_payment::{
    aggregate::BookingPayment, command::RequestPayment, error::BookingPaymentError, event::*,
};
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
        Ok(vec![BookingPaymentEvent::PaymentRequested(
            PaymentRequested {
                amount_cents: self.amount_cents,
                booking_id: self.booking_id,
                payment_id: self.payment_id,
            },
        )])
    }
}
