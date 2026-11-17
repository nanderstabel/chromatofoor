use crate::generated::domain::booking_payment::aggregate::BookingPayment;
use crate::generated::domain::booking_payment::event::PaymentRequested;
use chromatofoor::bounded_context::Event;
/// Event: `payment-requested` — Emitted when payment is requested for a booking.
impl Event<BookingPayment> for PaymentRequested {
    fn evolve(&self, _state: BookingPayment) -> BookingPayment {
        todo!(
            "implement Event::BookingPayment::evolve for PaymentRequested (return _state if stateless)"
        )
    }
}
