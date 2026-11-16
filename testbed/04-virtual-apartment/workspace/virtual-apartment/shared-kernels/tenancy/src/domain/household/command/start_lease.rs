use crate::generated::domain::household::aggregate::Household;
use crate::generated::domain::household::command::StartLease;
use crate::generated::domain::household::error::HouseholdError;
use crate::generated::domain::household::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Household> for StartLease {
    type Error = HouseholdError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Household) -> Result<Vec<HouseholdEvent>, Self::Error> {
        Ok(vec![
            crate::generated::domain::household::event::HouseholdEvent::LeaseStarted(
                crate::generated::domain::household::event::lease_started::LeaseStarted {
                    apartment_id: self.apartment_id,
                    start_date: self.start_date,
                    end_date: self.end_date,
                },
            ),
        ])
    }
}
