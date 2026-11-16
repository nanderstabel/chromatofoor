use crate::generated::application::read_models::account_summary::AccountSummary;
use crate::generated::domain::account::event::{
    account_opened::AccountOpened, funds_deposited::FundsDeposited, funds_withdrawn::FundsWithdrawn,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<AccountOpened> for AccountSummary {
    async fn project(&self, event: Envelope<AccountOpened>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.owner_name = Some(event.data.owner_name.clone());
            rec.current_balance = Some(event.data.balance);
        })?;
        Ok(())
    }
}

impl Project<FundsDeposited> for AccountSummary {
    async fn project(&self, event: Envelope<FundsDeposited>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.current_balance = Some(event.data.new_balance);
        })?;
        Ok(())
    }
}

impl Project<FundsWithdrawn> for AccountSummary {
    async fn project(&self, event: Envelope<FundsWithdrawn>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.current_balance = Some(event.data.new_balance);
        })?;
        Ok(())
    }
}
