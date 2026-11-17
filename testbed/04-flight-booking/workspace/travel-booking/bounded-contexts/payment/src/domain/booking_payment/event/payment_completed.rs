use crate::generated::domain::booking_payment::aggregate::BookingPayment;
use crate::generated::domain::booking_payment::event::PaymentCompleted;
use chromatofoor::bounded_context::Event;
/// Event: `payment-completed` — Emitted when payment for a booking succeeds.
impl Event<BookingPayment> for PaymentCompleted {
    fn evolve(&self, _state: BookingPayment) -> BookingPayment {
        todo!(
            "implement Event::BookingPayment::evolve for PaymentCompleted (return _state if stateless)"
        )
    }
}
