use crate::generated::domain::verification_session::aggregate::VerificationSession;
use crate::generated::domain::verification_session::event::PresentationVerified;
use chromatofoor::bounded_context::Event;

impl Event<VerificationSession> for PresentationVerified {
    fn evolve(&self, mut state: VerificationSession) -> VerificationSession {
        if let VerificationSession::Created(inner) = &mut state {
            inner.is_valid = Some(true);
            inner.status = Some("Verified".to_string());
        }
        state
    }
}
