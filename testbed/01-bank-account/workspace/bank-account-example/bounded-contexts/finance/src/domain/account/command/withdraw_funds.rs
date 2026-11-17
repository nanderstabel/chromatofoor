use crate::generated::domain::account::{
    aggregate::Account, command::WithdrawFunds, error::AccountError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `withdraw-funds` — Withdraws funds from an existing bank account when sufficient balance is available.
///
/// - Actors: `customer`
/// - Publishes: `funds-withdrawn`
/// - Invariants (`AccountError`):
///   - `insufficient-funds` — Account balance must cover the requested withdrawal amount.
impl Command<Account> for WithdrawFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        todo!("implement Command::Account::decide for WithdrawFunds")
    }
}
