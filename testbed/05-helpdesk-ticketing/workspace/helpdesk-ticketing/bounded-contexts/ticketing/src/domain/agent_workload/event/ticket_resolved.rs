use crate::generated::domain::agent_workload::aggregate::AgentWorkload;
use crate::generated::domain::ticket::event::ticket_resolved::TicketResolved;
use chromatofoor::bounded_context::Event;
impl Event<AgentWorkload> for TicketResolved {
    fn evolve(&self, state: AgentWorkload) -> AgentWorkload {
        state
    }
}
