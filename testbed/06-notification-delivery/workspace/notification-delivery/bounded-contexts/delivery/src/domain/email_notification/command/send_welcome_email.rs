use crate::generated::domain::email_notification::aggregate::EmailNotification;
use crate::generated::domain::email_notification::command::SendWelcomeEmail;
use crate::generated::domain::email_notification::error::EmailNotificationError;
use crate::generated::domain::email_notification::event::*;
use chromatofoor::bounded_context::Command;

impl Command<EmailNotification> for SendWelcomeEmail {
    type Error = EmailNotificationError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &EmailNotification,
    ) -> Result<Vec<EmailNotificationEvent>, Self::Error> {
        Ok(vec![crate::generated::domain::email_notification::event::welcome_email_sent::WelcomeEmailSent {
                notification_id: self.notification_id,
                user_id: self.user_id,
                email: self.email,
            }
            .into()])
    }
}
