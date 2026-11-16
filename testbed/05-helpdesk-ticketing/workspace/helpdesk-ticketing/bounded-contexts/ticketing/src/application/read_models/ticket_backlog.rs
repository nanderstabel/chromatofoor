#[allow(unused_imports)]
use crate::generated::application::read_models::ticket_backlog::{
    TicketBacklog, TicketBacklogRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::ticket::event::ticket_opened::TicketOpened>
    for TicketBacklog
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::ticket::event::ticket_opened::TicketOpened>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::ticket::event::message_added::MessageAdded>
    for TicketBacklog
{
    async fn project(
        &self,
        _event: Envelope<crate::generated::domain::ticket::event::message_added::MessageAdded>,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
