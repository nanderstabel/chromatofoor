use crate::generated::domain::account::aggregate::Account;
use crate::generated::domain::account::event::FundsDeposited;
use chromatofoor::bounded_context::Event;

impl Event<Account> for FundsDeposited {
    fn evolve(&self, mut state: Account) -> Account {
        if let Account::Created(inner) = &mut state {
            inner.balance = Some(self.new_balance);
        }
        state
    }
}
