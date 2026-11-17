#[allow(unused_imports)]
use crate::generated::application::read_models::account_summary::{
    AccountSummary, AccountSummaryRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `account-opened`
///
/// Rule: Insert a summary record with owner-name and current-balance equal to balance.
impl Project<crate::generated::domain::account::event::account_opened::AccountOpened>
    for AccountSummary
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::account::event::account_opened::AccountOpened>,
    ) -> Result<(), ViewError> {
        todo!("implement Project::AccountOpened::project for AccountSummary")
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-deposited`
///
/// Rule: Update current-balance to new-balance.
impl Project<crate::generated::domain::account::event::funds_deposited::FundsDeposited>
    for AccountSummary
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::account::event::funds_deposited::FundsDeposited>,
    ) -> Result<(), ViewError> {
        todo!("implement Project::FundsDeposited::project for AccountSummary")
    }
}
/// Read Model: `account-summary` — Materialized summary view of account owner and current balance.
/// Projection: `funds-withdrawn`
///
/// Rule: Update current-balance to new-balance.
impl Project<crate::generated::domain::account::event::funds_withdrawn::FundsWithdrawn>
    for AccountSummary
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::account::event::funds_withdrawn::FundsWithdrawn>,
    ) -> Result<(), ViewError> {
        todo!("implement Project::FundsWithdrawn::project for AccountSummary")
    }
}
