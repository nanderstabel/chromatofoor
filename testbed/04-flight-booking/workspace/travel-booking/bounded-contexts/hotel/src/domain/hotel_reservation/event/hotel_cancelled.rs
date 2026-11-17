use crate::generated::domain::hotel_reservation::aggregate::HotelReservation;
use crate::generated::domain::hotel_reservation::event::HotelCancelled;
use chromatofoor::bounded_context::Event;
impl Event<HotelReservation> for HotelCancelled {
    fn evolve(&self, _state: HotelReservation) -> HotelReservation {
        todo!(
            "implement Event::HotelReservation::evolve for HotelCancelled (return _state if stateless)"
        )
    }
}
