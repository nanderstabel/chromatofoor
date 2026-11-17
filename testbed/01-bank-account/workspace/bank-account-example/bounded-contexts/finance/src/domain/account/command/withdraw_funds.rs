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
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        let Account::Created(inner) = state else {
            return Ok(vec![]);
        };
        if self.amount > inner.balance {
            return Err(AccountError::InsufficientFunds);
        }
        Ok(vec![AccountEvent::FundsWithdrawn(FundsWithdrawn {
            amount: self.amount,
            new_balance: inner.balance - self.amount,
        })])
    }
}
