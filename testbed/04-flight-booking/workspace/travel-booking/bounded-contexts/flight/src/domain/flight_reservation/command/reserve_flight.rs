use crate::generated::domain::flight_reservation::{
    aggregate::FlightReservation, command::ReserveFlight, error::FlightReservationError, event::*,
};
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
        Ok(vec![FlightReservationEvent::FlightReserved(
            FlightReserved {
                booking_id: self.booking_id,
                flight_number: self.flight_number,
                reservation_id: self.reservation_id,
            },
        )])
    }
}
