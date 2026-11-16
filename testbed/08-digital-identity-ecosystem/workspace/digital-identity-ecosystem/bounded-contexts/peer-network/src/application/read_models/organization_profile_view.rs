#[allow(unused_imports)]
use crate::generated::application::read_models::organization_profile_view::{
    OrganizationProfileView, OrganizationProfileViewRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::organization_profile::event::profile_updated::ProfileUpdated>
    for OrganizationProfileView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::organization_profile::event::profile_updated::ProfileUpdated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::organization_profile::event::linked_presentation_attached::LinkedPresentationAttached,
> for OrganizationProfileView {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::organization_profile::event::linked_presentation_attached::LinkedPresentationAttached,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
