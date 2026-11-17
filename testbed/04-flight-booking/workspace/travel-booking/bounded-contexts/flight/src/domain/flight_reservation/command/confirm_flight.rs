use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::command::ConfirmFlight;
use crate::generated::domain::flight_reservation::error::FlightReservationError;
use crate::generated::domain::flight_reservation::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `confirm-flight` — Confirms a held flight reservation after payment succeeds.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `flight-confirmed`
impl Command<FlightReservation> for ConfirmFlight {
    type Error = FlightReservationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &FlightReservation,
    ) -> Result<Vec<FlightReservationEvent>, Self::Error> {
        todo!("implement Command::FlightReservation::decide for ConfirmFlight")
    }
}
