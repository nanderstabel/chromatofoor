use crate::generated::domain::booking_payment::{
    aggregate::BookingPayment, event::PaymentCompleted,
};
use chromatofoor::bounded_context::Event;
/// Event: `payment-completed` — Emitted when payment for a booking succeeds.
impl Event<BookingPayment> for PaymentCompleted {
    fn evolve(&self, mut state: BookingPayment) -> BookingPayment {
        if let BookingPayment::Created(ref mut inner) = state {
            inner.status = "Paid".to_owned();
        }
        state
    }
}
