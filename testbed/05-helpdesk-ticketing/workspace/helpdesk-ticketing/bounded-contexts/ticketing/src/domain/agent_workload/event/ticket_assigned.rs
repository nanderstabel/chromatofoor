use crate::generated::domain::agent_workload::aggregate::AgentWorkload;
use crate::generated::domain::ticket::event::ticket_assigned::TicketAssigned;
use chromatofoor::bounded_context::Event;
impl Event<AgentWorkload> for TicketAssigned {
    fn evolve(&self, state: AgentWorkload) -> AgentWorkload {
        state
    }
}
