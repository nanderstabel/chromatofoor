use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::event::FundsDeposited;
use chromatofoor::bounded_context::Event;
impl Event<Account> for FundsDeposited {
    fn evolve(&self, _state: Account) -> Account {
        todo!("implement Event::Account::evolve for FundsDeposited (return _state if stateless)")
    }
}
