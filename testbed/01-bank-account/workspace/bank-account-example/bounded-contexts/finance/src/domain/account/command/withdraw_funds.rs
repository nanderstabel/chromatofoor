use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::WithdrawFunds;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Account> for WithdrawFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        let current_balance = match aggregate {
            Account::Created(inner) => inner.balance.unwrap_or(0),
            Account::None => {
                return Err(AccountError::Validation("Not found".to_string()));
            }
        };
        if current_balance < self.amount {
            return Err(AccountError::Validation("Insufficient funds".to_string()));
        }
        Ok(vec![AccountEvent::FundsWithdrawn(FundsWithdrawn {
            amount: self.amount,
            new_balance: current_balance - self.amount,
        })])
    }
}
