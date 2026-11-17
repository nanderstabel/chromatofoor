use crate::generated::domain::booking_payment::aggregate::BookingPayment;
use crate::generated::domain::booking_payment::command::CompletePayment;
use crate::generated::domain::booking_payment::error::BookingPaymentError;
use crate::generated::domain::booking_payment::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `complete-payment` — Records successful payment capture by the traveler.
///
/// - Actors: `traveler`
/// - Publishes: `payment-completed`
impl Command<BookingPayment> for CompletePayment {
    type Error = BookingPaymentError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &BookingPayment,
    ) -> Result<Vec<BookingPaymentEvent>, Self::Error> {
        todo!("implement Command::BookingPayment::decide for CompletePayment")
    }
}
