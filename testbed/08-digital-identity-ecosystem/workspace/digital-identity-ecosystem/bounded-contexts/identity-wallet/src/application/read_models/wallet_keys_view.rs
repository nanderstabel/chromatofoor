#[allow(unused_imports)]
use crate::generated::application::read_models::wallet_keys_view::{
    WalletKeysView, WalletKeysViewRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl
    Project<
        crate::generated::domain::wallet_keychain::event::holder_key_generated::HolderKeyGenerated,
    > for WalletKeysView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::wallet_keychain::event::holder_key_generated::HolderKeyGenerated,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<
    crate::generated::domain::wallet_keychain::event::self_signed_credential_issued::SelfSignedCredentialIssued,
> for WalletKeysView {
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::wallet_keychain::event::self_signed_credential_issued::SelfSignedCredentialIssued,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
