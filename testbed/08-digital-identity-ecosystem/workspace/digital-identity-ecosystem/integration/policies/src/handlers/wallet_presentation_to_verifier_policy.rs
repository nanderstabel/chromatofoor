use crate::generated::handlers::wallet_presentation_to_verifier_policy::{
    WalletPresentationToVerifierPolicy, WalletPresentationToVerifierPolicyCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<
    identity_wallet::generated::domain::presentation_exchange::event::presentation_authorized::PresentationAuthorized,
> for WalletPresentationToVerifierPolicy {
    fn react(
        &self,
        _event: Envelope<
            identity_wallet::generated::domain::presentation_exchange::event::presentation_authorized::PresentationAuthorized,
        >,
    ) -> Vec<WalletPresentationToVerifierPolicyCommands> {
        Vec::new()
    }
}
