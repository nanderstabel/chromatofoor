use crate::generated::domain::organization_keyring::aggregate::OrganizationKeyring;
use crate::generated::domain::organization_keyring::command::GenerateSigningKey;
use crate::generated::domain::organization_keyring::error::OrganizationKeyringError;
use crate::generated::domain::organization_keyring::event::*;
use chromatofoor::bounded_context::Command;

impl Command<OrganizationKeyring> for GenerateSigningKey {
    type Error = OrganizationKeyringError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &OrganizationKeyring,
    ) -> Result<Vec<OrganizationKeyringEvent>, Self::Error> {
        Ok(vec![OrganizationKeyringEvent::SigningKeyGenerated(
            SigningKeyGenerated {
                key_id: "key-ed25519-01".to_string(),
                algorithm: self.algorithm,
                public_key_jwk: "{\"kty\":\"OKP\",\"crv\":\"Ed25519\",\"x\":\"12345\"}".to_string(),
            },
        )])
    }
}
