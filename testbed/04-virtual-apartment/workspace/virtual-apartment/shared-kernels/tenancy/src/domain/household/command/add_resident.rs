use crate::generated::domain::household::aggregate::Household;
use crate::generated::domain::household::command::AddResident;
use crate::generated::domain::household::error::HouseholdError;
use crate::generated::domain::household::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Household> for AddResident {
    type Error = HouseholdError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Household) -> Result<Vec<HouseholdEvent>, Self::Error> {
        Ok(vec![
            crate::generated::domain::household::event::HouseholdEvent::ResidentAdded(
                crate::generated::domain::household::event::resident_added::ResidentAdded {
                    resident_id: self.resident_id,
                },
            ),
        ])
    }
}
