use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::WithdrawFunds;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;
impl Command<Account> for WithdrawFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        todo!("implement Command::Account::decide for WithdrawFunds")
    }
}
