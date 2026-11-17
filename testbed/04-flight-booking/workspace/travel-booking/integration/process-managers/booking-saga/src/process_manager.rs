use crate::generated::*;
use chromatofoor::{
    architecturekit::Envelope,
    integration::{OnEvent, OnTimer},
};
impl BookingSaga {
    /// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
    ///
    /// - Ends When:
    ///   - `saga-completed` — Payment completed and flight confirmed, or compensating cancellations issued.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.completed.unwrap_or(false)
    }
}
/// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
/// Trigger Event: `flight` / `flight-reservation` / `flight-reserved`
///
/// Rule: Record booking-id and flight-reservation-id, then emit reserve-hotel with deterministic reservation-id hotel-{booking-id}.
/// - Emits: `hotel/hotel-reservation/reserve-hotel`
impl OnEvent<FlightReserved> for BookingSaga {
    fn on_event(&mut self, event: Envelope<FlightReserved>) -> Vec<BookingSagaCommands> {
        let booking_id = event.data.booking_id;
        let hotel_reservation_id: chromatofoor::Identifier = format!("hotel-{booking_id}").into();
        self.booking_id = Some(booking_id.clone());
        self.flight_reservation_id = Some(event.data.reservation_id);
        vec![
            ReserveHotel {
                booking_id,
                reservation_id: hotel_reservation_id,
            }
            .into(),
        ]
    }
}
/// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
/// Trigger Event: `hotel` / `hotel-reservation` / `hotel-reserved`
///
/// Rule: Record hotel-reservation-id, emit request-payment (payment-{booking-id}, 50000 cents), and arm payment-timeout.
/// - Emits: `payment/booking-payment/request-payment`
impl OnEvent<HotelReserved> for BookingSaga {
    fn on_event(&mut self, event: Envelope<HotelReserved>) -> Vec<BookingSagaCommands> {
        let booking_id = event.data.booking_id;
        let payment_id: chromatofoor::Identifier = format!("payment-{booking_id}").into();
        self.booking_id.get_or_insert_with(|| booking_id.clone());
        self.hotel_reservation_id = Some(event.data.reservation_id);
        self.payment_id = Some(payment_id.clone());
        vec![
            RequestPayment {
                amount_cents: 50_000,
                booking_id,
                payment_id,
            }
            .into(),
            PaymentTimeout::set().into(),
        ]
    }
}
/// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
/// Trigger Event: `payment` / `booking-payment` / `payment-completed`
///
/// Rule: Mark completed true, cancel payment-timeout, and emit confirm-flight.
/// - Emits: `flight/flight-reservation/confirm-flight`
impl OnEvent<PaymentCompleted> for BookingSaga {
    fn on_event(&mut self, event: Envelope<PaymentCompleted>) -> Vec<BookingSagaCommands> {
        self.completed = Some(true);
        let mut actions = vec![PaymentTimeout::cancel().into()];
        if let Some(flight_reservation_id) = self.flight_reservation_id.clone() {
            actions.push(
                ConfirmFlight {
                    booking_id: event.data.booking_id,
                    reservation_id: flight_reservation_id,
                }
                .into(),
            );
        }
        actions
    }
}
/// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
/// Trigger Event: `payment` / `booking-payment` / `payment-failed`
///
/// Rule: Mark completed true, cancel payment-timeout, and emit compensating cancel-flight and cancel-hotel commands.
/// - Emits: `flight/flight-reservation/cancel-flight`, `hotel/hotel-reservation/cancel-hotel`
impl OnEvent<PaymentFailed> for BookingSaga {
    fn on_event(&mut self, event: Envelope<PaymentFailed>) -> Vec<BookingSagaCommands> {
        self.completed = Some(true);
        let booking_id = event.data.booking_id;
        let mut actions = vec![PaymentTimeout::cancel().into()];
        if let Some(flight_reservation_id) = self.flight_reservation_id.clone() {
            actions.push(
                CancelFlight {
                    booking_id: booking_id.clone(),
                    reservation_id: flight_reservation_id,
                }
                .into(),
            );
        }
        if let Some(hotel_reservation_id) = self.hotel_reservation_id.clone() {
            actions.push(
                CancelHotel {
                    booking_id,
                    reservation_id: hotel_reservation_id,
                }
                .into(),
            );
        }
        actions
    }
}
/// Process Manager: `booking-saga` — Stateful travel booking saga coordinating flight holds, hotel reservations, payment deadlines, and compensating cancellations.
/// Trigger Timer: `payment-timeout` — Expires if payment is not completed within 15 minutes of hotel reservation.
///
/// Rule: Mark completed true and emit compensating cancel-flight and cancel-hotel commands.
/// - Emits: `flight/flight-reservation/cancel-flight`, `hotel/hotel-reservation/cancel-hotel`
impl OnTimer<PaymentTimeout> for BookingSaga {
    fn on_timer(&mut self) -> Vec<BookingSagaCommands> {
        self.completed = Some(true);
        let Some(booking_id) = self.booking_id.clone() else {
            return Vec::new();
        };
        let mut actions = Vec::new();
        if let Some(flight_reservation_id) = self.flight_reservation_id.clone() {
            actions.push(
                CancelFlight {
                    booking_id: booking_id.clone(),
                    reservation_id: flight_reservation_id,
                }
                .into(),
            );
        }
        if let Some(hotel_reservation_id) = self.hotel_reservation_id.clone() {
            actions.push(
                CancelHotel {
                    booking_id,
                    reservation_id: hotel_reservation_id,
                }
                .into(),
            );
        }
        actions
    }
}
