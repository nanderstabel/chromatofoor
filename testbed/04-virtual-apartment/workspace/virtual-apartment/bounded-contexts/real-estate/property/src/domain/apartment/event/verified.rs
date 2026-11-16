use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::event::Verified;
use chromatofoor::bounded_context::Event;

impl Event<Apartment> for Verified {
    fn evolve(&self, state: Apartment) -> Apartment {
        // TODO: Implement event evolution logic
        state
    }
}
