use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::event::AccountOpened;
use chromatofoor::bounded_context::Event;
impl Event<Account> for AccountOpened {
    fn evolve(&self, _state: Account) -> Account {
        todo!("implement Event::Account::evolve for AccountOpened (return _state if stateless)")
    }
}
