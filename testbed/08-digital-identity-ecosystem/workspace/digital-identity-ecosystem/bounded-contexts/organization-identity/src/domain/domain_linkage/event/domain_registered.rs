use crate::generated::domain::domain_linkage::aggregate::DomainLinkage;
use crate::generated::domain::domain_linkage::event::DomainRegistered;
use chromatofoor::bounded_context::Event;

impl Event<DomainLinkage> for DomainRegistered {
    fn evolve(&self, state: DomainLinkage) -> DomainLinkage {
        // TODO: Implement event evolution logic
        state
    }
}
