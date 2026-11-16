use crate::generated::domain::verification_session::aggregate::VerificationSession;
use crate::generated::domain::verification_session::command::InitiateVerification;
use crate::generated::domain::verification_session::error::VerificationSessionError;
use crate::generated::domain::verification_session::event::*;
use chromatofoor::bounded_context::Command;

impl Command<VerificationSession> for InitiateVerification {
    type Error = VerificationSessionError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &VerificationSession,
    ) -> Result<Vec<VerificationSessionEvent>, Self::Error> {
        Ok(vec![VerificationSessionEvent::VerificationInitiated(
            VerificationInitiated {
                session_id: "session-vp-001".to_string(),
                flow_id: self.flow_id,
                nonce: self.nonce,
            },
        )])
    }
}
