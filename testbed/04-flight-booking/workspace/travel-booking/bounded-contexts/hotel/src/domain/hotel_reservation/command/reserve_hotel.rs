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
        todo!("implement Command::HotelReservation::decide for ReserveHotel")
    }
}
