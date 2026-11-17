use crate::generated::domain::account::{
    aggregate::Account, command::DepositFunds, error::AccountError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `deposit-funds` — Deposits funds into an existing bank account (a zero-amount deposit is an idempotent no-op).
///
/// - Actors: `customer`, `bank-teller`
/// - Publishes: `funds-deposited`
/// - Invariants (`AccountError`):
///   - `insufficient-funds` — Account balance must cover the requested withdrawal amount.
impl Command<Account> for DepositFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, state))]
    async fn decide(self, state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        let Account::Created(inner) = state else {
            return Ok(vec![]);
        };
        if self.amount == 0 {
            return Ok(vec![]);
        }
        Ok(vec![AccountEvent::FundsDeposited(FundsDeposited {
            amount: self.amount,
            new_balance: inner.balance + self.amount,
        })])
    }
}
