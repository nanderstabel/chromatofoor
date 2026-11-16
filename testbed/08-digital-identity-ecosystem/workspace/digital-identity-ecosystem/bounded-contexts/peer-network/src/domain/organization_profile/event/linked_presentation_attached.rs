use crate::generated::domain::organization_profile::aggregate::OrganizationProfile;
use crate::generated::domain::organization_profile::event::LinkedPresentationAttached;
use chromatofoor::bounded_context::Event;

impl Event<OrganizationProfile> for LinkedPresentationAttached {
    fn evolve(&self, state: OrganizationProfile) -> OrganizationProfile {
        // TODO: Implement event evolution logic
        state
    }
}
