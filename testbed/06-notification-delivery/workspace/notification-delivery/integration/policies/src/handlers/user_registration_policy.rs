use crate::generated::handlers::user_registration_policy::{
    UserRegistrationPolicy, UserRegistrationPolicyCommands,
};
use chromatofoor::{Identifier, architecturekit::Envelope, integration::React};
use delivery::generated::domain::email_notification::command::send_welcome_email::SendWelcomeEmail;
use user_management::generated::domain::user::event::user_registered::UserRegistered;
use uuid::Uuid;

impl React<UserRegistered> for UserRegistrationPolicy {
    fn react(&self, event: Envelope<UserRegistered>) -> Vec<UserRegistrationPolicyCommands> {
        let Ok(notification_id) = Identifier::try_new(Uuid::new_v4().to_string()) else {
            return Vec::new();
        };

        vec![
            SendWelcomeEmail {
                notification_id,
                user_id: event.data.user_id,
                email: event.data.email,
            }
            .into(),
        ]
    }
}
