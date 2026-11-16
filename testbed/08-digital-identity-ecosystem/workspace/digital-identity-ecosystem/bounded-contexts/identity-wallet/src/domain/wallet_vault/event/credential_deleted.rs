use crate::generated::domain::wallet_vault::aggregate::WalletVault;
use crate::generated::domain::wallet_vault::event::CredentialDeleted;
use chromatofoor::bounded_context::Event;

impl Event<WalletVault> for CredentialDeleted {
    fn evolve(&self, mut state: WalletVault) -> WalletVault {
        if let WalletVault::Created(inner) = &mut state {
            let current = inner.stored_credentials_count.unwrap_or(1);
            inner.stored_credentials_count = Some(if current > 0 { current - 1 } else { 0 });
        }
        state
    }
}
