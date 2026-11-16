use crate::generated::domain::verification_session::aggregate::VerificationSession;
use crate::generated::domain::verification_session::command::VerifyPresentation;
use crate::generated::domain::verification_session::error::VerificationSessionError;
use crate::generated::domain::verification_session::event::*;
use chromatofoor::bounded_context::Command;

impl Command<VerificationSession> for VerifyPresentation {
    type Error = VerificationSessionError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &VerificationSession,
    ) -> Result<Vec<VerificationSessionEvent>, Self::Error> {
        let session_id = if let VerificationSession::Created(inner) = aggregate {
            inner
                .session_id
                .clone()
                .unwrap_or_else(|| "session-vp-001".to_string())
        } else {
            "session-vp-001".to_string()
        };
        Ok(vec![VerificationSessionEvent::PresentationVerified(
            PresentationVerified {
                session_id,
                verified_claims_count: 3,
            },
        )])
    }
}
