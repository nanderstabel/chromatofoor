use crate::generated::domain::hotel_reservation::aggregate::HotelReservation;
use crate::generated::domain::hotel_reservation::command::ReserveHotel;
use crate::generated::domain::hotel_reservation::error::HotelReservationError;
use crate::generated::domain::hotel_reservation::event::*;
use chromatofoor::bounded_context::Command;
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
