use crate::generated::*;
use chromatofoor::{
    architecturekit::Envelope,
    integration::{OnEvent, OnTimer},
};
impl BookingSaga {
    #[must_use]
    pub fn has_ended(&self) -> bool {
        todo!("implement BookingSaga::has_ended")
    }
}
impl OnEvent<FlightReserved> for BookingSaga {
    fn on_event(&mut self, _event: Envelope<FlightReserved>) -> Vec<BookingSagaCommands> {
        todo!("implement OnEvent::FlightReserved::on_event for BookingSaga")
    }
}
impl OnEvent<HotelReserved> for BookingSaga {
    fn on_event(&mut self, _event: Envelope<HotelReserved>) -> Vec<BookingSagaCommands> {
        todo!(
            "implement OnEvent::HotelReserved::on_event for BookingSaga (declared timers: set=[PaymentTimeout], cancel=[])"
        )
    }
}
impl OnEvent<PaymentCompleted> for BookingSaga {
    fn on_event(&mut self, _event: Envelope<PaymentCompleted>) -> Vec<BookingSagaCommands> {
        todo!(
            "implement OnEvent::PaymentCompleted::on_event for BookingSaga (declared timers: set=[], cancel=[PaymentTimeout])"
        )
    }
}
impl OnEvent<PaymentFailed> for BookingSaga {
    fn on_event(&mut self, _event: Envelope<PaymentFailed>) -> Vec<BookingSagaCommands> {
        todo!(
            "implement OnEvent::PaymentFailed::on_event for BookingSaga (declared timers: set=[], cancel=[PaymentTimeout])"
        )
    }
}
impl OnTimer<PaymentTimeout> for BookingSaga {
    fn on_timer(&mut self) -> Vec<BookingSagaCommands> {
        todo!("implement OnTimer::PaymentTimeout::on_timer for BookingSaga")
    }
}
