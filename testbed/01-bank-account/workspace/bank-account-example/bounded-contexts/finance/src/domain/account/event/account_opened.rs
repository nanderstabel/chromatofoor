use crate::generated::domain::account::{aggregate::Account, event::AccountOpened};
use chromatofoor::bounded_context::Event;
/// Event: `account-opened` — Emitted when a new bank account has been opened.
impl Event<Account> for AccountOpened {
    fn evolve(&self, _state: Account) -> Account {
        todo!("implement Event::Account::evolve for AccountOpened (return _state if stateless)")
    }
}
