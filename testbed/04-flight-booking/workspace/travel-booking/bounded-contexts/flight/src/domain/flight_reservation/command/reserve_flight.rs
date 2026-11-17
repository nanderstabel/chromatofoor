use crate::generated::domain::flight_reservation::aggregate::FlightReservation;
use crate::generated::domain::flight_reservation::command::ReserveFlight;
use crate::generated::domain::flight_reservation::error::FlightReservationError;
use crate::generated::domain::flight_reservation::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `reserve-flight` — Places an initial hold on a flight seat for a booking.
///
/// - Actors: `traveler`, `booking-coordinator`
/// - Publishes: `flight-reserved`
impl Command<FlightReservation> for ReserveFlight {
    type Error = FlightReservationError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(
        self,
        _state: &FlightReservation,
    ) -> Result<Vec<FlightReservationEvent>, Self::Error> {
        todo!("implement Command::FlightReservation::decide for ReserveFlight")
    }
}
