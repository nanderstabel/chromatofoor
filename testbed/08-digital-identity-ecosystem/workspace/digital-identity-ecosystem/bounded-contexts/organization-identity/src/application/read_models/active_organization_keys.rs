#[allow(unused_imports)]
use crate::generated::application::read_models::active_organization_keys::{
    ActiveOrganizationKeys, ActiveOrganizationKeysRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<
    crate::generated::domain::organization_keyring::event::signing_key_generated::SigningKeyGenerated,
> for ActiveOrganizationKeys {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::organization_keyring::event::signing_key_generated::SigningKeyGenerated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::organization_keyring::event::signing_key_rotated::SigningKeyRotated,
> for ActiveOrganizationKeys {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::organization_keyring::event::signing_key_rotated::SigningKeyRotated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
