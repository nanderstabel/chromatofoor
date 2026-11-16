use crate::generated::handlers::domain_linkage_to_peer_network_policy::{
    DomainLinkageToPeerNetworkPolicy, DomainLinkageToPeerNetworkPolicyCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<
    organization_identity::generated::domain::domain_linkage::event::domain_registered::DomainRegistered,
> for DomainLinkageToPeerNetworkPolicy {
    fn react(
        &self,
        _event: Envelope<
            organization_identity::generated::domain::domain_linkage::event::domain_registered::DomainRegistered,
        >,
    ) -> Vec<DomainLinkageToPeerNetworkPolicyCommands> {
        Vec::new()
    }
}
