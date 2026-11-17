use crate::generated::domain::hotel_reservation::{
    aggregate::HotelReservation, event::HotelCancelled,
};
use chromatofoor::bounded_context::Event;
/// Event: `hotel-cancelled` — Emitted when a hotel room reservation is cancelled.
impl Event<HotelReservation> for HotelCancelled {
    fn evolve(&self, _state: HotelReservation) -> HotelReservation {
        todo!(
            "implement Event::HotelReservation::evolve for HotelCancelled (return _state if stateless)"
        )
    }
}
