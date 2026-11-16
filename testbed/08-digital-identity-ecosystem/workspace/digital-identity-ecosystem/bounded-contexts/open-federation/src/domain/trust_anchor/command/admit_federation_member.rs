use crate::generated::domain::trust_anchor::aggregate::TrustAnchor;
use crate::generated::domain::trust_anchor::command::AdmitFederationMember;
use crate::generated::domain::trust_anchor::error::TrustAnchorError;
use crate::generated::domain::trust_anchor::event::*;
use chromatofoor::bounded_context::Command;

impl Command<TrustAnchor> for AdmitFederationMember {
    type Error = TrustAnchorError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &TrustAnchor) -> Result<Vec<TrustAnchorEvent>, Self::Error> {
        Ok(vec![])
    }
}
