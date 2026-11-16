use crate::generated::handlers::send_welcome_email::SendWelcomeEmail;
use chromatofoor::{
    architecturekit::{DomainError, Envelope},
    integration::Handle,
};
#[derive(Clone, Default, Debug)]
pub struct SendWelcomeEmailDeps {}
impl Handle<tenancy::generated::domain::household::event::formed::Formed> for SendWelcomeEmail {
    async fn handle(
        &self,
        _event: Envelope<tenancy::generated::domain::household::event::formed::Formed>,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}
