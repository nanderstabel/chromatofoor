#[allow(unused_imports)]
use crate::generated::application::read_models::verified_domains::{
    VerifiedDomains, VerifiedDomainsRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::domain_linkage::event::domain_registered::DomainRegistered>
    for VerifiedDomains
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::domain_linkage::event::domain_registered::DomainRegistered,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::domain_linkage::event::linkage_verified::LinkageVerified>
    for VerifiedDomains
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::domain_linkage::event::linkage_verified::LinkageVerified,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
