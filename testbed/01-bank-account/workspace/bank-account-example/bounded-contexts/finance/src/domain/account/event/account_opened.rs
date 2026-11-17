use crate::generated::domain::account::{
    aggregate::{Account, AccountInner},
    event::AccountOpened,
};
use chromatofoor::bounded_context::Event;
/// Event: `account-opened` — Emitted when a new bank account has been opened.
impl Event<Account> for AccountOpened {
    fn evolve(&self, _state: Account) -> Account {
        Account::Created(AccountInner {
            balance: self.balance,
            owner_name: self.owner_name.clone(),
        })
    }
}
