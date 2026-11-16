use crate::generated::domain::issuance_session::aggregate::IssuanceSession;
use crate::generated::domain::issuance_session::event::BindingProofAccepted;
use chromatofoor::bounded_context::Event;

impl Event<IssuanceSession> for BindingProofAccepted {
    fn evolve(&self, mut state: IssuanceSession) -> IssuanceSession {
        if let IssuanceSession::Created(inner) = &mut state {
            inner.holder_binding_key = Some(self.holder_binding_key.clone());
            inner.status = Some("ProofAccepted".to_string());
        }
        state
    }
}
