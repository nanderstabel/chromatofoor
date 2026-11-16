use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::event::MessageAdded;
use chromatofoor::bounded_context::Event;

impl Event<Ticket> for MessageAdded {
    fn evolve(&self, state: Ticket) -> Ticket {
        // TODO: Implement event evolution logic
        state
    }
}
