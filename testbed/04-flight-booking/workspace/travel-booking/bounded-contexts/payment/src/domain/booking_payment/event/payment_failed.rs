use crate::generated::domain::booking_payment::{aggregate::BookingPayment, event::PaymentFailed};
use chromatofoor::bounded_context::Event;
/// Event: `payment-failed` — Emitted when payment for a booking is declined.
impl Event<BookingPayment> for PaymentFailed {
    fn evolve(&self, mut state: BookingPayment) -> BookingPayment {
        if let BookingPayment::Created(ref mut inner) = state {
            inner.status = "Failed".to_owned();
        }
        state
    }
}
