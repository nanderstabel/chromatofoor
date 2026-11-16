#[allow(unused_imports)]
use crate::generated::application::read_models::verification_flows::{
    VerificationFlows, VerificationFlowsRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::verification_flow::event::flow_defined::FlowDefined>
    for VerificationFlows
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::verification_flow::event::flow_defined::FlowDefined,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
