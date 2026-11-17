#[allow(unused_imports)]
use crate::generated::application::read_models::account_summary::{
    AccountSummary, AccountSummaryRecord,
};
use crate::generated::domain::account::event::AccountOpened;
use crate::generated::domain::account::event::FundsDeposited;
use crate::generated::domain::account::event::FundsWithdrawn;
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `account-opened`
///
/// Rule: Insert a summary record keyed by event subject ID with owner-name and current-balance equal to balance.
impl Project<AccountOpened> for AccountSummary {
    async fn project(&self, event: Envelope<AccountOpened>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.owner_name.clone_from(&event.data.owner_name);
            record.current_balance = event.data.balance;
        })?;
        Ok(())
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-deposited`
///
/// Rule: Update current-balance to new-balance for the event subject ID.
impl Project<FundsDeposited> for AccountSummary {
    async fn project(&self, event: Envelope<FundsDeposited>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.current_balance = event.data.new_balance;
        })?;
        Ok(())
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-withdrawn`
///
/// Rule: Update current-balance to new-balance for the event subject ID.
impl Project<FundsWithdrawn> for AccountSummary {
    async fn project(&self, event: Envelope<FundsWithdrawn>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |record| {
            record.current_balance = event.data.new_balance;
        })?;
        Ok(())
    }
}
