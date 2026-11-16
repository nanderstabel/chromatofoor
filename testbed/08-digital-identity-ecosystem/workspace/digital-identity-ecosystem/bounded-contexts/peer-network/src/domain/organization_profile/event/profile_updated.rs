use crate::generated::domain::organization_profile::aggregate::OrganizationProfile;
use crate::generated::domain::organization_profile::event::ProfileUpdated;
use chromatofoor::bounded_context::Event;

impl Event<OrganizationProfile> for ProfileUpdated {
    fn evolve(&self, state: OrganizationProfile) -> OrganizationProfile {
        // TODO: Implement event evolution logic
        state
    }
}
