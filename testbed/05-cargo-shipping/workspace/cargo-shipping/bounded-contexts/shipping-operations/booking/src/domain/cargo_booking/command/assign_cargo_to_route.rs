use crate::generated::domain::cargo_booking::aggregate::CargoBooking;
use crate::generated::domain::cargo_booking::command::AssignCargoToRoute;
use crate::generated::domain::cargo_booking::error::CargoBookingError;
use crate::generated::domain::cargo_booking::event::*;
use chromatofoor::bounded_context::Command;
impl Command<CargoBooking> for AssignCargoToRoute {
    type Error = CargoBookingError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &CargoBooking) -> Result<Vec<CargoBookingEvent>, Self::Error> {
        todo!("implement Command::CargoBooking::decide for AssignCargoToRoute")
    }
}
