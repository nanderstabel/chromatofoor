use crate::generated::domain::verification_flow::aggregate::VerificationFlow;
use crate::generated::domain::verification_flow::event::FlowDefined;
use chromatofoor::bounded_context::Event;

impl Event<VerificationFlow> for FlowDefined {
    fn evolve(&self, state: VerificationFlow) -> VerificationFlow {
        // TODO: Implement event evolution logic
        state
    }
}
