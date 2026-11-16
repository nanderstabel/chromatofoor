use crate::generated::domain::domain_linkage::aggregate::DomainLinkage;
use crate::generated::domain::domain_linkage::command::RegisterDomain;
use crate::generated::domain::domain_linkage::error::DomainLinkageError;
use crate::generated::domain::domain_linkage::event::*;
use chromatofoor::bounded_context::Command;

impl Command<DomainLinkage> for RegisterDomain {
    type Error = DomainLinkageError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &DomainLinkage,
    ) -> Result<Vec<DomainLinkageEvent>, Self::Error> {
        Ok(vec![])
    }
}
