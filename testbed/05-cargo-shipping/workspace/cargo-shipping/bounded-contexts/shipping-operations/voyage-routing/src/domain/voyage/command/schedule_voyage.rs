use crate::generated::domain::voyage::{
    aggregate::Voyage, command::ScheduleVoyage, error::VoyageError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `schedule-voyage` — Schedules a new vessel voyage between departure and arrival UN/LOCODE ports.
///
/// - Actors: `planner`
/// - Publishes: `voyage-scheduled`
impl Command<Voyage> for ScheduleVoyage {
    type Error = VoyageError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Voyage) -> Result<Vec<VoyageEvent>, Self::Error> {
        Ok(vec![VoyageEvent::VoyageScheduled(VoyageScheduled {
            arrival_locode: self.arrival_locode,
            departure_locode: self.departure_locode,
            voyage_number: self.voyage_number,
        })])
    }
}
