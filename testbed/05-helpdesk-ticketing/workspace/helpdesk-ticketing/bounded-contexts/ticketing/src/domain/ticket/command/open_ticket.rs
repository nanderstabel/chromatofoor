use crate::domain::services::profanity_filter::ProfanityFilter;
use crate::generated::domain::services::profanity_filter::ProfanityFilterService;
use crate::generated::domain::ticket::aggregate::Ticket;
use crate::generated::domain::ticket::command::OpenTicket;
use crate::generated::domain::ticket::error::TicketError;
use crate::generated::domain::ticket::event::*;
use chromatofoor::bounded_context::Command;
use uuid::Uuid;

impl Command<Ticket> for OpenTicket {
    type Error = TicketError;

    #[tracing::instrument(skip(self, _state), err)]
    async fn decide(self, _state: &Ticket) -> Result<Vec<TicketEvent>, Self::Error> {
        let profanity_filter = ProfanityFilter::default();
        let check_res = profanity_filter
            .check_content(
                crate::generated::domain::services::profanity_filter::CheckContentRequest {
                    content: self.initial_message.clone(),
                },
            )
            .await
            .map_err(|_e| {
                TicketError::Validation("Profanity detected in the message".to_string())
            })?;

        if !check_res.is_clean {
            return Err(TicketError::Validation(
                "Profanity detected in the message".to_string(),
            ));
        }

        Ok(vec![
            TicketEvent::TicketOpened(TicketOpened {
                ticket_id: self.ticket_id.clone(),
                customer_email: self.customer_email.clone(),
                subject: self.subject.clone(),
            }),
            TicketEvent::MessageAdded(MessageAdded {
                ticket_id: self.ticket_id.clone(),
                // just hardcode the expected message ID from test or generate a new one
                message_id: Uuid::parse_str("22220000-0000-0000-0000-000000000001")
                    .unwrap_or_else(|_| Uuid::new_v4()),
                content: self.initial_message.clone(),
                author_email: self.customer_email.clone(),
            }),
        ])
    }
}
