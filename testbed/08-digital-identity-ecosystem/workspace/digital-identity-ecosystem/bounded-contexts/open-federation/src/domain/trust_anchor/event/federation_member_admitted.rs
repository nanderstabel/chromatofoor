use crate::generated::domain::trust_anchor::aggregate::TrustAnchor;
use crate::generated::domain::trust_anchor::event::FederationMemberAdmitted;
use chromatofoor::bounded_context::Event;

impl Event<TrustAnchor> for FederationMemberAdmitted {
    fn evolve(&self, state: TrustAnchor) -> TrustAnchor {
        // TODO: Implement event evolution logic
        state
    }
}
