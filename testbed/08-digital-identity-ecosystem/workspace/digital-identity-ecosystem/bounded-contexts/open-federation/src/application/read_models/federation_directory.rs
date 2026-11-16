#[allow(unused_imports)]
use crate::generated::application::read_models::federation_directory::{
    FederationDirectory, FederationDirectoryRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<
        crate::generated::domain::trust_anchor::event::ecosystem_initialized::EcosystemInitialized,
    > for FederationDirectory
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::trust_anchor::event::ecosystem_initialized::EcosystemInitialized,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::trust_anchor::event::federation_member_admitted::FederationMemberAdmitted,
> for FederationDirectory {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::trust_anchor::event::federation_member_admitted::FederationMemberAdmitted,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
