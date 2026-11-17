use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::command::CancelFlight;
use crate::generated::domain::flight_reservation::error::FlightReservationError;
use crate::generated::domain::flight_reservation::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `cancel-flight` — Cancels a held flight reservation when payment fails or times out.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `flight-cancelled`
impl Command<FlightReservation> for CancelFlight {
    type Error = FlightReservationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &FlightReservation,
    ) -> Result<Vec<FlightReservationEvent>, Self::Error> {
        todo!("implement Command::FlightReservation::decide for CancelFlight")
    }
}
