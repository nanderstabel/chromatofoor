use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::event::Unregistered;
use chromatofoor::bounded_context::Event;

impl Event<Apartment> for Unregistered {
    fn evolve(&self, state: Apartment) -> Apartment {
        // TODO: Implement event evolution logic
        state
    }
}
