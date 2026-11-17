use crate::generated::domain::account::{aggregate::Account, event::FundsWithdrawn};
use chromatofoor::bounded_context::Event;
/// Event: `funds-withdrawn` — Emitted when funds have been withdrawn from a bank account.
impl Event<Account> for FundsWithdrawn {
    fn evolve(&self, mut state: Account) -> Account {
        if let Account::Created(ref mut inner) = state {
            inner.balance = self.new_balance;
        }
        state
    }
}
