use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::command::ReleaseSeat;
use crate::generated::domain::flight::error::FlightError;
use crate::generated::domain::flight::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Flight> for ReleaseSeat {
    type Error = FlightError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Flight) -> Result<Vec<FlightEvent>, Self::Error> {
        use crate::generated::domain::flight::event::SeatReleased;
        Ok(vec![
            SeatReleased {
                booking_id: self.booking_id,
            }
            .into(),
        ])
    }
}
