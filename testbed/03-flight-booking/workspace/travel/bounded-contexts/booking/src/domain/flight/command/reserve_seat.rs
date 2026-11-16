use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::command::ReserveSeat;
use crate::generated::domain::flight::event::FlightEvent;
use chromatofoor::bounded_context::Command;

impl Command<Flight> for ReserveSeat {
    type Error = crate::generated::domain::flight::error::FlightError;

    #[tracing::instrument(skip(self, state), err)]
    async fn decide(self, state: &Flight) -> Result<Vec<FlightEvent>, Self::Error> {
        let available_seats = match state {
            Flight::Created(inner) => inner.total_seats - inner.reserved_seats,
            Flight::None => {
                use crate::generated::domain::flight::error::FlightError;
                return Err(FlightError::Validation("Flight does not exist".to_string()));
            }
        };

        if available_seats > 0 {
            use crate::generated::domain::flight::event::SeatReserved;
            Ok(vec![
                SeatReserved {
                    booking_id: self.booking_id,
                    flight_id: self.flight_id,
                }
                .into(),
            ])
        } else {
            use crate::generated::domain::flight::event::SeatReservationFailed;
            Ok(vec![
                SeatReservationFailed {
                    booking_id: self.booking_id,
                    flight_id: self.flight_id,
                }
                .into(),
            ])
        }
    }
}
