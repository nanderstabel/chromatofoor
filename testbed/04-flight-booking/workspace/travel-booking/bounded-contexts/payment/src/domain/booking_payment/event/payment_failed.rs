use crate::generated::domain::booking_payment::aggregate::BookingPayment;
use crate::generated::domain::booking_payment::event::PaymentFailed;
use chromatofoor::bounded_context::Event;
/// Event: `payment-failed` — Emitted when payment for a booking is declined.
impl Event<BookingPayment> for PaymentFailed {
    fn evolve(&self, _state: BookingPayment) -> BookingPayment {
        todo!(
            "implement Event::BookingPayment::evolve for PaymentFailed (return _state if stateless)"
        )
    }
}
