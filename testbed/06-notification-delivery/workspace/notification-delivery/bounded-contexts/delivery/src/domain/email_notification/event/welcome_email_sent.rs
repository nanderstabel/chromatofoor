use crate::generated::domain::email_notification::aggregate::EmailNotification;
use crate::generated::domain::email_notification::event::WelcomeEmailSent;
use chromatofoor::bounded_context::Event;

impl Event<EmailNotification> for WelcomeEmailSent {
    fn evolve(&self, state: EmailNotification) -> EmailNotification {
        // TODO: Implement event evolution logic
        state
    }
}
