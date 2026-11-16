use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::event::TicketResolved;
use chromatofoor::bounded_context::Event;

impl Event<Ticket> for TicketResolved {
    fn evolve(&self, state: Ticket) -> Ticket {
        // TODO: Implement event evolution logic
        state
    }
}
