use crate::generated::domain::organization_keyring::aggregate::OrganizationKeyring;
use crate::generated::domain::organization_keyring::event::SigningKeyRotated;
use chromatofoor::bounded_context::Event;

impl Event<OrganizationKeyring> for SigningKeyRotated {
    fn evolve(&self, mut state: OrganizationKeyring) -> OrganizationKeyring {
        if let OrganizationKeyring::Created(inner) = &mut state {
            inner.active_key_id = Some(self.new_key_id.clone());
        }
        state
    }
}
