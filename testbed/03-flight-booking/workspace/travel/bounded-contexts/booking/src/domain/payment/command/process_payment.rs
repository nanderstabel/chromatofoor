use crate::generated::domain::payment::aggregate::Payment;
use crate::generated::domain::payment::command::ProcessPayment;
use crate::generated::domain::payment::error::PaymentError;
use crate::generated::domain::payment::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Payment> for ProcessPayment {
    type Error = PaymentError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Payment) -> Result<Vec<PaymentEvent>, Self::Error> {
        Ok(vec![
            PaymentInitiated {
                booking_id: self.booking_id,
                flight_id: self.flight_id,
                amount: self.amount,
            }
            .into(),
        ])
    }
}
