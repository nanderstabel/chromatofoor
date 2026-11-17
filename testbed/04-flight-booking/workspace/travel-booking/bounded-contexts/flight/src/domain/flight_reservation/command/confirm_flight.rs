use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, command::ConfirmFlight, error::FlightReservationError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `confirm-flight` — Confirms a held flight reservation after payment succeeds.
///
/// - Actors: `booking-coordinator`
/// - Publishes: `flight-confirmed`
impl Command<FlightReservation> for ConfirmFlight {
    type Error = FlightReservationError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(
        self,
        state: &FlightReservation,
    ) -> Result<Vec<FlightReservationEvent>, Self::Error> {
        if state.is_none() {
            return Ok(vec![]);
        }
        Ok(vec![FlightReservationEvent::FlightConfirmed(
            FlightConfirmed {
                booking_id: self.booking_id,
                reservation_id: self.reservation_id,
            },
        )])
    }
}
