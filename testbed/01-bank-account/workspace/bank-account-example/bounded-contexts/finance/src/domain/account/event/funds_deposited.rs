use crate::generated::domain::account::{aggregate::Account, event::FundsDeposited};
use chromatofoor::bounded_context::Event;
/// Event: `funds-deposited` — Emitted when positive funds have been deposited into a bank account.
impl Event<Account> for FundsDeposited {
    fn evolve(&self, mut state: Account) -> Account {
        if let Account::Created(ref mut inner) = state {
            inner.balance = self.new_balance;
        }
        state
    }
}
