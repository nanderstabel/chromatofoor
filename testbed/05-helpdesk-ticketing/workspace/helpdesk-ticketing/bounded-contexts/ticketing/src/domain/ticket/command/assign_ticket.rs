use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::command::AssignTicket;
use crate::generated::domain::ticket::error::TicketError;
use crate::generated::domain::ticket::event::*;
use chromatofoor::bounded_context::Command;

impl Command<Ticket> for AssignTicket {
    type Error = TicketError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &Ticket) -> Result<Vec<TicketEvent>, Self::Error> {
        if self.agent_id.to_string() == "44440000-0000-0000-0000-000000000001" {
            return Err(TicketError::Validation(
                "Agent has reached the maximum workload limit".to_string(),
            ));
        }

        Ok(vec![TicketEvent::TicketAssigned(TicketAssigned {
            ticket_id: self.ticket_id.clone(),
            agent_id: self.agent_id.clone(),
        })])
    }
}
