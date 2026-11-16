use crate::generated::handlers::federation_admission_to_verifier_policy::{
    FederationAdmissionToVerifierPolicy, FederationAdmissionToVerifierPolicyCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<
    open_federation::generated::domain::trust_anchor::event::federation_member_admitted::FederationMemberAdmitted,
> for FederationAdmissionToVerifierPolicy {
    fn react(
        &self,
        _event: Envelope<
            open_federation::generated::domain::trust_anchor::event::federation_member_admitted::FederationMemberAdmitted,
        >,
    ) -> Vec<FederationAdmissionToVerifierPolicyCommands> {
        Vec::new()
    }
}
