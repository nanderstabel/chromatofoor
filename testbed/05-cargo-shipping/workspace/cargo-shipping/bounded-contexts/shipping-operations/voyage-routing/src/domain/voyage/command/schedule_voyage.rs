use crate::generated::domain::voyage::aggregate::Voyage;
use crate::generated::domain::voyage::command::ScheduleVoyage;
use crate::generated::domain::voyage::error::VoyageError;
use crate::generated::domain::voyage::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `schedule-voyage` — Schedules a new vessel voyage between departure and arrival UN/LOCODE ports.
///
/// - Actors: `planner`
/// - Publishes: `voyage-scheduled`
impl Command<Voyage> for ScheduleVoyage {
    type Error = VoyageError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Voyage) -> Result<Vec<VoyageEvent>, Self::Error> {
        todo!("implement Command::Voyage::decide for ScheduleVoyage")
    }
}
