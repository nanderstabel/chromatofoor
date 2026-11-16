#[allow(unused_imports)]
use crate::generated::application::read_models::sent_emails::{SentEmails, SentEmailsRecord};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<
        crate::generated::domain::email_notification::event::welcome_email_sent::WelcomeEmailSent,
    > for SentEmails
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::email_notification::event::welcome_email_sent::WelcomeEmailSent,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
