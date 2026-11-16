use crate::generated::domain::user::aggregate::User;
use crate::generated::domain::user::command::RegisterUser;
use crate::generated::domain::user::error::UserError;
use crate::generated::domain::user::event::*;
use chromatofoor::bounded_context::Command;

impl Command<User> for RegisterUser {
    type Error = UserError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &User) -> Result<Vec<UserEvent>, Self::Error> {
        Ok(vec![
            UserRegistered {
                user_id: self.user_id,
                email: self.email,
            }
            .into(),
        ])
    }
}
