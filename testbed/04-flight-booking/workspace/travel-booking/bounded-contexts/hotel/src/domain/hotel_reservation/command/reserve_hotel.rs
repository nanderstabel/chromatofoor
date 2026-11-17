use crate::generated::domain::hotel_reservation::{
    aggregate::HotelReservation, command::ReserveHotel, error::HotelReservationError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `reserve-hotel` — Reserves a hotel room for a booking.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `hotel-reserved`
impl Command<HotelReservation> for ReserveHotel {
    type Error = HotelReservationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &HotelReservation,
    ) -> Result<Vec<HotelReservationEvent>, Self::Error> {
        Ok(vec![HotelReservationEvent::HotelReserved(HotelReserved {
            booking_id: self.booking_id,
            reservation_id: self.reservation_id,
        })])
    }
}
