use crate::generated::handlers::lease_rejection_policy::{
    LeaseRejectionPolicy, LeaseRejectionPolicyCommands,
};
use chromatofoor::{architecturekit::Envelope, integration::React};
impl React<tenancy::generated::domain::household::event::lease_requested::LeaseRequested>
    for LeaseRejectionPolicy
{
    fn react(
        &self,
        _event: Envelope<
            tenancy::generated::domain::household::event::lease_requested::LeaseRequested,
        >,
    ) -> Vec<LeaseRejectionPolicyCommands> {
        Vec::new()
    }
}
