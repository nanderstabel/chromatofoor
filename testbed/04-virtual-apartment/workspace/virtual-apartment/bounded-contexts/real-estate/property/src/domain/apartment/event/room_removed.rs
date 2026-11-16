use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::event::RoomRemoved;
use chromatofoor::bounded_context::Event;

impl Event<Apartment> for RoomRemoved {
    fn evolve(&self, state: Apartment) -> Apartment {
        // TODO: Implement event evolution logic
        state
    }
}
