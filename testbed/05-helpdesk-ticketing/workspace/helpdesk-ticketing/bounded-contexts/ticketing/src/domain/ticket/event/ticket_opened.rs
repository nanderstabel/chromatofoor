use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::event::TicketOpened;
use chromatofoor::bounded_context::Event;

impl Event<Ticket> for TicketOpened {
    fn evolve(&self, state: Ticket) -> Ticket {
        // TODO: Implement event evolution logic
        state
    }
}
