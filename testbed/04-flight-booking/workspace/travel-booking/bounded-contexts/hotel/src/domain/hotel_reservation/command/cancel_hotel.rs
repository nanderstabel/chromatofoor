use crate::generated::domain::hotel_reservation::{
    aggregate::HotelReservation, command::CancelHotel, error::HotelReservationError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `cancel-hotel` — Cancels a hotel reservation during saga compensation.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `hotel-cancelled`
impl Command<HotelReservation> for CancelHotel {
    type Error = HotelReservationError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(
        self,
        state: &HotelReservation,
    ) -> Result<Vec<HotelReservationEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![HotelReservationEvent::HotelCancelled(
            HotelCancelled {
                booking_id: self.booking_id,
                reservation_id: self.reservation_id,
            },
        )])
    }
}
