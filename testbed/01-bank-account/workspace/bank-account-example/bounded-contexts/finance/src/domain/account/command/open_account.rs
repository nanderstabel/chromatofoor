use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::OpenAccount;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;
/// Command: `open-account` — Opens a new bank account with an initial non-negative balance.
///
/// - Actors: `customer`, `bank-teller`
/// - Publishes: `account-opened`
/// - Invariants (`AccountError`):
///   - `insufficient-funds` — Account balance must cover the requested withdrawal amount.
impl Command<Account> for OpenAccount {
    type Error = AccountError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        todo!("implement Command::Account::decide for OpenAccount")
    }
}
