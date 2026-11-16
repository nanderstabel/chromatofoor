use crate::generated::domain::organization_keyring::aggregate::OrganizationKeyring;
use crate::generated::domain::organization_keyring::event::SigningKeyGenerated;
use chromatofoor::bounded_context::Event;

impl Event<OrganizationKeyring> for SigningKeyGenerated {
    fn evolve(&self, _state: OrganizationKeyring) -> OrganizationKeyring {
        OrganizationKeyring::Created(
            crate::generated::domain::organization_keyring::aggregate::OrganizationKeyringInner {
                active_key_id: Some(self.key_id.clone()),
                algorithm: Some(self.algorithm.clone()),
                status: Some("Active".to_string()),
            },
        )
    }
}
