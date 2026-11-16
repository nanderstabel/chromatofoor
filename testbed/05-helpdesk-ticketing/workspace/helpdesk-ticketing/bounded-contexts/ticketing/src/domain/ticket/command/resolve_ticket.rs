use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::command::ResolveTicket;
use crate::generated::domain::ticket::error::TicketError;
use crate::generated::domain::ticket::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Ticket> for ResolveTicket {
    type Error = TicketError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Ticket) -> Result<Vec<TicketEvent>, Self::Error> {
        Ok(vec![TicketEvent::TicketResolved(TicketResolved {
            ticket_id: self.ticket_id.clone(),
        })])
    }
}
