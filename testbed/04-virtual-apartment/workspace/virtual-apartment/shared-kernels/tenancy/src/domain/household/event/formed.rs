use crate::generated::domain::household::aggregate::Household;
use crate::generated::domain::household::event::Formed;
use chromatofoor::bounded_context::Event;

impl Event<Household> for Formed {
    fn evolve(&self, state: Household) -> Household {
        // TODO: Implement event evolution logic
        state
    }
}
