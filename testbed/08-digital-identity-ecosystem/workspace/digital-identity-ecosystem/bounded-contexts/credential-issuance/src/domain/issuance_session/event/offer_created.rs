use crate::generated::domain::issuance_session::aggregate::IssuanceSession;
use crate::generated::domain::issuance_session::event::OfferCreated;
use chromatofoor::bounded_context::Event;

impl Event<IssuanceSession> for OfferCreated {
    fn evolve(&self, _state: IssuanceSession) -> IssuanceSession {
        IssuanceSession::Created(
            crate::generated::domain::issuance_session::aggregate::IssuanceSessionInner {
                session_id: Some(self.session_id.clone()),
                holder_did: Some(self.holder_did.clone()),
                template_id: Some(self.template_id.clone()),
                holder_binding_key: None,
                status: Some("Offered".to_string()),
            },
        )
    }
}
