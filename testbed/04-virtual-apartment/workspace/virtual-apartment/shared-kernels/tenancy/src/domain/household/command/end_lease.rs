use crate::generated::domain::household::aggregate::Household;
use crate::generated::domain::household::command::EndLease;
use crate::generated::domain::household::error::HouseholdError;
use crate::generated::domain::household::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Household> for EndLease {
    type Error = HouseholdError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Household) -> Result<Vec<HouseholdEvent>, Self::Error> {
        Ok(vec![
            crate::generated::domain::household::event::HouseholdEvent::LeaseEnded(
                crate::generated::domain::household::event::lease_ended::LeaseEnded {},
            ),
        ])
    }
}
