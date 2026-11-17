use crate::generated::domain::hotel_reservation::{
    aggregate::{HotelReservation, HotelReservationInner},
    event::HotelReserved,
};
use chromatofoor::bounded_context::Event;
/// Event: `hotel-reserved` — Emitted when a hotel room is reserved for a booking.
impl Event<HotelReservation> for HotelReserved {
    fn evolve(&self, _state: HotelReservation) -> HotelReservation {
        HotelReservation::Created(HotelReservationInner {
            booking_id: self.booking_id.clone(),
            reservation_id: self.reservation_id.clone(),
            status: "Reserved".to_owned(),
        })
    }
}
