use crate::generated::domain::hotel_reservation::{
    aggregate::HotelReservation, event::HotelCancelled,
};
use chromatofoor::bounded_context::Event;
/// Event: `hotel-cancelled` — Emitted when a hotel room reservation is cancelled.
impl Event<HotelReservation> for HotelCancelled {
    fn evolve(&self, mut state: HotelReservation) -> HotelReservation {
        if let HotelReservation::Created(ref mut inner) = state {
            inner.status = "Cancelled".to_owned();
        }
        state
    }
}
