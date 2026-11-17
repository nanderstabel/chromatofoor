use crate::generated::domain::hotel_reservation::aggregate::HotelReservation;
use crate::generated::domain::hotel_reservation::event::HotelReserved;
use chromatofoor::bounded_context::Event;
/// Event: `hotel-reserved` — Emitted when a hotel room is reserved for a booking.
impl Event<HotelReservation> for HotelReserved {
    fn evolve(&self, _state: HotelReservation) -> HotelReservation {
        todo!(
            "implement Event::HotelReservation::evolve for HotelReserved (return _state if stateless)"
        )
    }
}
