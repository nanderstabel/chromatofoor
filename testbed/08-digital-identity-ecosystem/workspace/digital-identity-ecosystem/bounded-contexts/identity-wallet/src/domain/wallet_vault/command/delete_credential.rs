use crate::generated::domain::wallet_vault::aggregate::WalletVault;
use crate::generated::domain::wallet_vault::command::DeleteCredential;
use crate::generated::domain::wallet_vault::error::WalletVaultError;
use crate::generated::domain::wallet_vault::event::*;
use chromatofoor::bounded_context::Command;

impl Command<WalletVault> for DeleteCredential {
    type Error = WalletVaultError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(self, _aggregate: &WalletVault) -> Result<Vec<WalletVaultEvent>, Self::Error> {
        Ok(vec![WalletVaultEvent::CredentialDeleted(
            CredentialDeleted {
                credential_id: self.credential_id,
            },
        )])
    }
}
