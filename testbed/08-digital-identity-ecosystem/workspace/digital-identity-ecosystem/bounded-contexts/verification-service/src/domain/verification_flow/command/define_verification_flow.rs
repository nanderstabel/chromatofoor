use crate::generated::domain::verification_flow::aggregate::VerificationFlow;
use crate::generated::domain::verification_flow::command::DefineVerificationFlow;
use crate::generated::domain::verification_flow::error::VerificationFlowError;
use crate::generated::domain::verification_flow::event::*;
use chromatofoor::bounded_context::Command;

impl Command<VerificationFlow> for DefineVerificationFlow {
    type Error = VerificationFlowError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &VerificationFlow,
    ) -> Result<Vec<VerificationFlowEvent>, Self::Error> {
        Ok(vec![])
    }
}
