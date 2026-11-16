use crate::generated::domain::household::aggregate::Household;
use crate::generated::domain::household::command::RemoveResident;
use crate::generated::domain::household::error::HouseholdError;
use crate::generated::domain::household::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Household> for RemoveResident {
    type Error = HouseholdError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Household) -> Result<Vec<HouseholdEvent>, Self::Error> {
        Ok(vec![
            crate::generated::domain::household::event::HouseholdEvent::ResidentRemoved(
                crate::generated::domain::household::event::resident_removed::ResidentRemoved {
                    resident_id: self.resident_id,
                },
            ),
        ])
    }
}
