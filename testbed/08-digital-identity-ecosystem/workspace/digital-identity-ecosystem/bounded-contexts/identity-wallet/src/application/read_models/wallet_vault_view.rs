use crate::generated::application::read_models::wallet_vault_view::WalletVaultView;
use crate::generated::domain::wallet_vault::event::{
    credential_deleted::CredentialDeleted, credential_stored::CredentialStored,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<CredentialStored> for WalletVaultView {
    async fn project(&self, event: Envelope<CredentialStored>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.total_credentials = Some(rec.total_credentials.unwrap_or(0) + 1);
        })?;
        Ok(())
    }
}

impl Project<CredentialDeleted> for WalletVaultView {
    async fn project(&self, event: Envelope<CredentialDeleted>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            let current = rec.total_credentials.unwrap_or(1);
            rec.total_credentials = Some(if current > 0 { current - 1 } else { 0 });
        })?;
        Ok(())
    }
}
