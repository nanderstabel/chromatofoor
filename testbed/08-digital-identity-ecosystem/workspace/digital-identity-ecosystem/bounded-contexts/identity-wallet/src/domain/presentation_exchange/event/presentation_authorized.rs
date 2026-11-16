use crate::generated::domain::presentation_exchange::aggregate::PresentationExchange;
use crate::generated::domain::presentation_exchange::event::PresentationAuthorized;
use chromatofoor::bounded_context::Event;

impl Event<PresentationExchange> for PresentationAuthorized {
    fn evolve(&self, state: PresentationExchange) -> PresentationExchange {
        // TODO: Implement event evolution logic
        state
    }
}
