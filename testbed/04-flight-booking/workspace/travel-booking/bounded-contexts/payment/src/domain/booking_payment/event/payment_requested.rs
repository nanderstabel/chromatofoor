use crate::generated::domain::booking_payment::{
    aggregate::{BookingPayment, BookingPaymentInner},
    event::PaymentRequested,
};
use chromatofoor::bounded_context::Event;
/// Event: `payment-requested` — Emitted when payment is requested for a booking.
impl Event<BookingPayment> for PaymentRequested {
    fn evolve(&self, _state: BookingPayment) -> BookingPayment {
        BookingPayment::Created(BookingPaymentInner {
            amount_cents: self.amount_cents,
            booking_id: self.booking_id.clone(),
            payment_id: self.payment_id.clone(),
            status: "Pending".to_owned(),
        })
    }
}
