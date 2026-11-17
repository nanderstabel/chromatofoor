use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::event::FundsWithdrawn;
use chromatofoor::bounded_context::Event;
impl Event<Account> for FundsWithdrawn {
    fn evolve(&self, _state: Account) -> Account {
        todo!("implement Event::Account::evolve for FundsWithdrawn (return _state if stateless)")
    }
}
