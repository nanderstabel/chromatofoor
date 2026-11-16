use crate::generated::domain::issuance_session::aggregate::IssuanceSession;
use crate::generated::domain::issuance_session::event::CredentialIssued;
use chromatofoor::bounded_context::Event;

impl Event<IssuanceSession> for CredentialIssued {
    fn evolve(&self, mut state: IssuanceSession) -> IssuanceSession {
        if let IssuanceSession::Created(inner) = &mut state {
            inner.status = Some("Issued".to_string());
        }
        state
    }
}
