use crate::generated::domain::payment::aggregate::Payment;
use crate::generated::domain::payment::command::CompletePayment;
use crate::generated::domain::payment::error::PaymentError;
use crate::generated::domain::payment::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Payment> for CompletePayment {
    type Error = PaymentError;

    #[tracing::instrument(skip(self, _state), err)]
    async fn decide(self, _state: &Payment) -> Result<Vec<PaymentEvent>, Self::Error> {
        if self.success {
            Ok(vec![
                PaymentProcessed {
                    booking_id: self.booking_id,
                    flight_id: self.flight_id,
                    amount: self.amount,
                }
                .into(),
            ])
        } else {
            Ok(vec![
                PaymentFailed {
                    booking_id: self.booking_id,
                    flight_id: self.flight_id,
                    reason: "Payment declined by gateway".to_string(),
                }
                .into(),
            ])
        }
    }
}
