use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::command::DepositFunds;
use crate::generated::domain::account::error::AccountError;
use crate::generated::domain::account::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Account> for DepositFunds {
    type Error = AccountError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(self, aggregate: &Account) -> Result<Vec<AccountEvent>, Self::Error> {
        if let Account::Created(inner) = aggregate {
            let current_balance = inner.balance.unwrap_or(0);
            let new_balance = current_balance + self.amount;

            Ok(vec![AccountEvent::FundsDeposited(FundsDeposited {
                amount: self.amount,
                new_balance,
            })])
        } else {
            Err(AccountError::Validation(
                "Account does not exist".to_string(),
            ))
        }
    }
}
