use crate::generated::domain::organization_keyring::aggregate::OrganizationKeyring;
use crate::generated::domain::organization_keyring::command::RotateSigningKey;
use crate::generated::domain::organization_keyring::error::OrganizationKeyringError;
use crate::generated::domain::organization_keyring::event::*;
use chromatofoor::bounded_context::Command;

impl Command<OrganizationKeyring> for RotateSigningKey {
    type Error = OrganizationKeyringError;
    #[tracing::instrument(skip(self, aggregate), err)]
    async fn decide(
        self,
        aggregate: &OrganizationKeyring,
    ) -> Result<Vec<OrganizationKeyringEvent>, Self::Error> {
        let prev_key = if let OrganizationKeyring::Created(inner) = aggregate {
            inner
                .active_key_id
                .clone()
                .unwrap_or_else(|| "key-ed25519-01".to_string())
        } else {
            "key-ed25519-01".to_string()
        };
        Ok(vec![OrganizationKeyringEvent::SigningKeyRotated(
            SigningKeyRotated {
                previous_key_id: prev_key,
                new_key_id: "key-es256-02".to_string(),
            },
        )])
    }
}
