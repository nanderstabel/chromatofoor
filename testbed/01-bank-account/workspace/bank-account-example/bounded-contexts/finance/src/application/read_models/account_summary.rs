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
/// Rule: Insert a summary record with owner-name and current-balance equal to balance.
impl Project<AccountOpened> for AccountSummary {
    async fn project(&self, _event: Envelope<AccountOpened>) -> Result<(), ViewError> {
        todo!("implement Project::AccountOpened::project for AccountSummary")
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-deposited`
///
/// Rule: Update current-balance to new-balance.
impl Project<FundsDeposited> for AccountSummary {
    async fn project(&self, _event: Envelope<FundsDeposited>) -> Result<(), ViewError> {
        todo!("implement Project::FundsDeposited::project for AccountSummary")
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-withdrawn`
///
/// Rule: Update current-balance to new-balance.
impl Project<FundsWithdrawn> for AccountSummary {
    async fn project(&self, _event: Envelope<FundsWithdrawn>) -> Result<(), ViewError> {
        todo!("implement Project::FundsWithdrawn::project for AccountSummary")
    }
}
