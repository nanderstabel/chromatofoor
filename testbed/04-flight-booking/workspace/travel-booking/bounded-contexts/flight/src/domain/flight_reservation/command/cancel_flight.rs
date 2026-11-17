use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, command::CancelFlight, error::FlightReservationError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `cancel-flight` — Cancels a held flight reservation when payment fails or times out.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `flight-cancelled`
impl Command<FlightReservation> for CancelFlight {
    type Error = FlightReservationError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(
        self,
        state: &FlightReservation,
    ) -> Result<Vec<FlightReservationEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![FlightReservationEvent::FlightCancelled(
            FlightCancelled {
                booking_id: self.booking_id,
                reservation_id: self.reservation_id,
            },
        )])
    }
}
