use crate::generated::domain::issuance_session::aggregate::IssuanceSession;
use crate::generated::domain::issuance_session::command::CreateOffer;
use crate::generated::domain::issuance_session::error::IssuanceSessionError;
use crate::generated::domain::issuance_session::event::*;
use chromatofoor::bounded_context::Command;

impl Command<IssuanceSession> for CreateOffer {
    type Error = IssuanceSessionError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &IssuanceSession,
    ) -> Result<Vec<IssuanceSessionEvent>, Self::Error> {
        Ok(vec![IssuanceSessionEvent::OfferCreated(OfferCreated {
            session_id: "session-oid4vci-999".to_string(),
            template_id: self.template_id,
            holder_did: self.holder_did,
        })])
    }
}
