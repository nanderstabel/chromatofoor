use crate::generated::handlers::issuer_delivery_to_wallet_policy::{
    IssuerDeliveryToWalletPolicy, IssuerDeliveryToWalletPolicyCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<
    credential_issuance::generated::domain::issuance_session::event::credential_issued::CredentialIssued,
> for IssuerDeliveryToWalletPolicy {
    fn react(
        &self,
        _event: Envelope<
            credential_issuance::generated::domain::issuance_session::event::credential_issued::CredentialIssued,
        >,
    ) -> Vec<IssuerDeliveryToWalletPolicyCommands> {
        Vec::new()
    }
}
