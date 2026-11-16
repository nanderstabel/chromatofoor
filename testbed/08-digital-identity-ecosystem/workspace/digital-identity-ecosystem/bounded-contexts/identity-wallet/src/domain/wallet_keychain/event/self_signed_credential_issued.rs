use crate::generated::domain::wallet_keychain::aggregate::WalletKeychain;
use crate::generated::domain::wallet_keychain::event::SelfSignedCredentialIssued;
use chromatofoor::bounded_context::Event;

impl Event<WalletKeychain> for SelfSignedCredentialIssued {
    fn evolve(&self, mut state: WalletKeychain) -> WalletKeychain {
        if let WalletKeychain::Created(inner) = &mut state {
            inner.key_count = Some(inner.key_count.unwrap_or(0) + 1);
        }
        state
    }
}
