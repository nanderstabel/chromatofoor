use crate::generated::domain::presentation_exchange::aggregate::PresentationExchange;
use crate::generated::domain::presentation_exchange::event::RequestReceived;
use chromatofoor::bounded_context::Event;

impl Event<PresentationExchange> for RequestReceived {
    fn evolve(&self, state: PresentationExchange) -> PresentationExchange {
        // TODO: Implement event evolution logic
        state
    }
}
