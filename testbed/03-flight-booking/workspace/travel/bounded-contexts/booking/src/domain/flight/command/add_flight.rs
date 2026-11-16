use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::command::AddFlight;
use crate::generated::domain::flight::error::FlightError;
use crate::generated::domain::flight::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Flight> for AddFlight {
    type Error = FlightError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Flight) -> Result<Vec<FlightEvent>, Self::Error> {
        use crate::generated::domain::flight::event::FlightAdded;
        Ok(vec![
            FlightAdded {
                flight_id: self.flight_id,
                total_seats: self.total_seats,
            }
            .into(),
        ])
    }
}
