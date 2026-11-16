use crate::generated::domain::wallet_vault::aggregate::WalletVault;
use crate::generated::domain::wallet_vault::event::CredentialStored;
use chromatofoor::bounded_context::Event;

impl Event<WalletVault> for CredentialStored {
    fn evolve(&self, mut state: WalletVault) -> WalletVault {
        let prev_count = if let WalletVault::Created(inner) = &mut state {
            inner.stored_credentials_count.unwrap_or(0)
        } else {
            0
        };
        state = WalletVault::Created(
            crate::generated::domain::wallet_vault::aggregate::WalletVaultInner {
                last_updated: Some("2026-09-11T00:00:00Z".to_string()),
                stored_credentials_count: Some(prev_count + 1),
            },
        );
        state
    }
}
