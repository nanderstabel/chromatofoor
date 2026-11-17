use crate::generated::domain::account::{aggregate::Account, event::FundsDeposited};
use chromatofoor::bounded_context::Event;
/// Event: `funds-deposited` — Emitted when positive funds have been deposited into a bank account.
impl Event<Account> for FundsDeposited {
    fn evolve(&self, _state: Account) -> Account {
        todo!("implement Event::Account::evolve for FundsDeposited (return _state if stateless)")
    }
}
