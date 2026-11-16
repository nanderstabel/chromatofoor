#[allow(unused_imports)]
use crate::generated::application::read_models::issuance_activity_view::{
    IssuanceActivityView, IssuanceActivityViewRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::issuance_session::event::offer_created::OfferCreated>
    for IssuanceActivityView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::issuance_session::event::offer_created::OfferCreated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::issuance_session::event::credential_issued::CredentialIssued>
    for IssuanceActivityView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::issuance_session::event::credential_issued::CredentialIssued,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::issuance_session::event::binding_proof_accepted::BindingProofAccepted,
> for IssuanceActivityView {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::issuance_session::event::binding_proof_accepted::BindingProofAccepted,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
