use crate::generated::domain::hotel_reservation::aggregate::HotelReservation;
use crate::generated::domain::hotel_reservation::command::CancelHotel;
use crate::generated::domain::hotel_reservation::error::HotelReservationError;
use crate::generated::domain::hotel_reservation::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `cancel-hotel` — Cancels a hotel reservation during saga compensation.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `hotel-cancelled`
impl Command<HotelReservation> for CancelHotel {
    type Error = HotelReservationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &HotelReservation,
    ) -> Result<Vec<HotelReservationEvent>, Self::Error> {
        todo!("implement Command::HotelReservation::decide for CancelHotel")
    }
}
