use crate::generated::domain::user::aggregate::User;
use crate::generated::domain::user::event::UserRegistered;
use chromatofoor::bounded_context::Event;

impl Event<User> for UserRegistered {
    fn evolve(&self, state: User) -> User {
        // TODO: Implement event evolution logic
        state
    }
}
