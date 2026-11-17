use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::DepositFunds;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `deposit-funds` — Deposits funds into an existing bank account (a zero-amount deposit is an idempotent no-op).
///
/// - Actors: `customer`, `bank-teller`
/// - Publishes: `funds-deposited`
/// - Invariants (`AccountError`):
///   - `insufficient-funds` — Account balance must cover the requested withdrawal amount.
impl Command<Account> for DepositFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        todo!("implement Command::Account::decide for DepositFunds")
    }
}
