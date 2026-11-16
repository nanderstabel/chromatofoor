use crate::generated::domain::wallet_keychain::aggregate::WalletKeychain;
use crate::generated::domain::wallet_keychain::event::HolderKeyGenerated;
use chromatofoor::bounded_context::Event;

impl Event<WalletKeychain> for HolderKeyGenerated {
    fn evolve(&self, _state: WalletKeychain) -> WalletKeychain {
        WalletKeychain::Created(
            crate::generated::domain::wallet_keychain::aggregate::WalletKeychainInner {
                active_holder_did: Some(self.holder_did.clone()),
                key_count: Some(1),
            },
        )
    }
}
