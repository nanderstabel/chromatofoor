#[allow(unused_imports)]
use crate::generated::application::read_models::token_status_lists::{
    TokenStatusLists, TokenStatusListsRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<
        crate::generated::domain::token_status_list::event::status_list_created::StatusListCreated,
    > for TokenStatusLists
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::token_status_list::event::status_list_created::StatusListCreated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::token_status_list::event::credential_index_revoked::CredentialIndexRevoked,
> for TokenStatusLists {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::token_status_list::event::credential_index_revoked::CredentialIndexRevoked,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
