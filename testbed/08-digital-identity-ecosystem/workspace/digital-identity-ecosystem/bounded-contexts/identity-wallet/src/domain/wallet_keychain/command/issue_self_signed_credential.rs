use crate::generated::domain::wallet_keychain::aggregate::WalletKeychain;
use crate::generated::domain::wallet_keychain::command::IssueSelfSignedCredential;
use crate::generated::domain::wallet_keychain::error::WalletKeychainError;
use crate::generated::domain::wallet_keychain::event::*;
use chromatofoor::bounded_context::Command;

impl Command<WalletKeychain> for IssueSelfSignedCredential {
    type Error = WalletKeychainError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &WalletKeychain,
    ) -> Result<Vec<WalletKeychainEvent>, Self::Error> {
        Ok(vec![WalletKeychainEvent::SelfSignedCredentialIssued(
            SelfSignedCredentialIssued {
                credential_id: "self-vc-999".to_string(),
                subject: self.subject,
            },
        )])
    }
}
