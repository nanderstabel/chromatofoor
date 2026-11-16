use crate::generated::domain::wallet_keychain::aggregate::WalletKeychain;
use crate::generated::domain::wallet_keychain::command::GenerateHolderKey;
use crate::generated::domain::wallet_keychain::error::WalletKeychainError;
use crate::generated::domain::wallet_keychain::event::*;
use chromatofoor::bounded_context::Command;

impl Command<WalletKeychain> for GenerateHolderKey {
    type Error = WalletKeychainError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &WalletKeychain,
    ) -> Result<Vec<WalletKeychainEvent>, Self::Error> {
        Ok(vec![WalletKeychainEvent::HolderKeyGenerated(
            HolderKeyGenerated {
                holder_did: "did:jwk:eyJrdHkiOiJPS1AiLCJjcnYiOiJFZDI1NTE5In0".to_string(),
                algorithm: self.algorithm,
            },
        )])
    }
}
