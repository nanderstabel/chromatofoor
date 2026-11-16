use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::event::AccountOpened;
use chromatofoor::bounded_context::Event;

impl Event<Account> for AccountOpened {
    fn evolve(&self, _state: Account) -> Account {
        Account::Created(crate::generated::domain::account::aggregate::AccountInner {
            owner_name: Some(self.owner_name.clone()),
            balance: Some(self.balance),
        })
    }
}
