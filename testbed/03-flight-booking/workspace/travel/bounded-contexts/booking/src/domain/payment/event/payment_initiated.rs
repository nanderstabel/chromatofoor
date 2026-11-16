use crate::generated::domain::payment::aggregate::Payment;
use crate::generated::domain::payment::event::PaymentInitiated;
use chromatofoor::bounded_context::Event;

impl Event<Payment> for PaymentInitiated {
    fn evolve(&self, state: Payment) -> Payment {
        // TODO: Implement event evolution logic
        state
    }
}
