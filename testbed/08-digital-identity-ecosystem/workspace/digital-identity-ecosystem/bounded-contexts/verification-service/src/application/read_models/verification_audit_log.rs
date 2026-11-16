#[allow(unused_imports)]
use crate::generated::application::read_models::verification_audit_log::{
    VerificationAuditLog, VerificationAuditLogRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<
    crate::generated::domain::verification_session::event::verification_initiated::VerificationInitiated,
> for VerificationAuditLog {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::verification_session::event::verification_initiated::VerificationInitiated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::verification_session::event::presentation_verified::PresentationVerified,
> for VerificationAuditLog {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::verification_session::event::presentation_verified::PresentationVerified,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
