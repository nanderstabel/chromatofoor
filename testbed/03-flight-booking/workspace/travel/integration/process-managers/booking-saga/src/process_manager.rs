use crate::generated::*;

impl BookingSaga {
    #[must_use]
    pub fn has_ended(&self) -> bool {
        matches!(self.status.as_deref(), Some("Confirmed" | "Failed"))
    }
}

impl chromatofoor::integration::OnEvent<BookingInitialized> for BookingSaga {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<BookingInitialized>,
    ) -> Vec<BookingSagaCommands> {
        self.booking_id = Some(event.data.booking_id.clone());
        self.flight_id = Some(event.data.flight_id.clone());
        self.amount = Some(event.data.amount);
        self.status = Some("ReservingSeat".to_owned());

        vec![
            ReserveSeat {
                booking_id: event.data.booking_id,
                flight_id: event.data.flight_id,
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnEvent<SeatReserved> for BookingSaga {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<SeatReserved>,
    ) -> Vec<BookingSagaCommands> {
        self.status = Some("ProcessingPayment".to_owned());

        vec![
            PaymentTimeout::set().into(),
            ProcessPayment {
                booking_id: event.data.booking_id,
                flight_id: event.data.flight_id,
                amount: self.amount.unwrap_or_default(),
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnEvent<SeatReservationFailed> for BookingSaga {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<SeatReservationFailed>,
    ) -> Vec<BookingSagaCommands> {
        self.status = Some("Failed".to_owned());

        vec![
            FailBooking {
                booking_id: event.data.booking_id,
                reason: "Seat reservation failed".to_owned(),
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnEvent<PaymentProcessed> for BookingSaga {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<PaymentProcessed>,
    ) -> Vec<BookingSagaCommands> {
        self.status = Some("Confirmed".to_owned());

        vec![
            PaymentTimeout::cancel().into(),
            ConfirmBooking {
                booking_id: event.data.booking_id,
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnEvent<PaymentFailed> for BookingSaga {
    fn on_event(
        &mut self,
        event: chromatofoor::architecturekit::Envelope<PaymentFailed>,
    ) -> Vec<BookingSagaCommands> {
        self.status = Some("Failed".to_owned());

        vec![
            PaymentTimeout::cancel().into(),
            ReleaseSeat {
                booking_id: event.data.booking_id.clone(),
                flight_id: self.flight_id.clone().unwrap_or_else(|| "unknown".into()),
            }
            .into(),
            FailBooking {
                booking_id: event.data.booking_id,
                reason: event.data.reason,
            }
            .into(),
        ]
    }
}

impl chromatofoor::integration::OnTimer<PaymentTimeout> for BookingSaga {
    fn on_timer(&mut self) -> Vec<BookingSagaCommands> {
        self.status = Some("Failed".to_owned());
        let booking_id = self.booking_id.clone().unwrap_or_else(|| "unknown".into());
        let flight_id = self.flight_id.clone().unwrap_or_else(|| "unknown".into());

        vec![
            ReleaseSeat {
                booking_id: booking_id.clone(),
                flight_id,
            }
            .into(),
            FailBooking {
                booking_id,
                reason: "Payment timed out".to_owned(),
            }
            .into(),
        ]
    }
}
