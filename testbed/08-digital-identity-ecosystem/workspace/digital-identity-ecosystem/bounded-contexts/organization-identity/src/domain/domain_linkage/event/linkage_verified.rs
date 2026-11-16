use crate::generated::domain::domain_linkage::aggregate::DomainLinkage;
use crate::generated::domain::domain_linkage::event::LinkageVerified;
use chromatofoor::bounded_context::Event;

impl Event<DomainLinkage> for LinkageVerified {
    fn evolve(&self, state: DomainLinkage) -> DomainLinkage {
        // TODO: Implement event evolution logic
        state
    }
}
