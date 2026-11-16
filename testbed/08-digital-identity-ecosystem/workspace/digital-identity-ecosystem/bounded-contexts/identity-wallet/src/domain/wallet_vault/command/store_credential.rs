use crate::generated::domain::wallet_vault::aggregate::WalletVault;
use crate::generated::domain::wallet_vault::command::StoreCredential;
use crate::generated::domain::wallet_vault::error::WalletVaultError;
use crate::generated::domain::wallet_vault::event::*;
use chromatofoor::bounded_context::Command;

impl Command<WalletVault> for StoreCredential {
    type Error = WalletVaultError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &WalletVault) -> Result<Vec<WalletVaultEvent>, Self::Error> {
        Ok(vec![WalletVaultEvent::CredentialStored(CredentialStored {
            credential_id: self.credential_id,
            issuer_did: self.issuer_did,
        })])
    }
}
