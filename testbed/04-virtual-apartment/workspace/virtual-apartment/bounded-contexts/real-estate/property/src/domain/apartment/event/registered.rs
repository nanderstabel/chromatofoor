use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::event::Registered;
use chromatofoor::bounded_context::Event;

impl Event<Apartment> for Registered {
    fn evolve(&self, _state: Apartment) -> Apartment {
        Apartment::Created(
            crate::generated::domain::apartment::aggregate::ApartmentInner { rooms: vec![] },
        )
    }
}
