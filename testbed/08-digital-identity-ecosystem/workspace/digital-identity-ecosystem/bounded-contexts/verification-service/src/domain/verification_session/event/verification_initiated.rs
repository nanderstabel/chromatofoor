use crate::generated::domain::verification_session::aggregate::VerificationSession;
use crate::generated::domain::verification_session::event::VerificationInitiated;
use chromatofoor::bounded_context::Event;

impl Event<VerificationSession> for VerificationInitiated {
    fn evolve(&self, _state: VerificationSession) -> VerificationSession {
        VerificationSession::Created(
            crate::generated::domain::verification_session::aggregate::VerificationSessionInner {
                session_id: Some(self.session_id.clone()),
                flow_id: Some(self.flow_id.clone()),
                nonce: Some(self.nonce.clone()),
                is_valid: None,
                status: Some("Initiated".to_string()),
            },
        )
    }
}
