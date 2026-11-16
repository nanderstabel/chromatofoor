use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::OpenAccount;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Account> for OpenAccount {
    type Error = AccountError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        if !aggregate.is_none() {
            return Err(AccountError::Validation(
                "Account already opened".to_string(),
            ));
        }

        Ok(vec![AccountEvent::AccountOpened(AccountOpened {
            owner_name: self.owner_name,
            balance: self.initial_balance,
        })])
    }
}
