use crate::generated::domain::apartment::aggregate::Apartment;
use crate::generated::domain::apartment::event::RoomAdded;
use chromatofoor::bounded_context::Event;

impl Event<Apartment> for RoomAdded {
    fn evolve(&self, mut state: Apartment) -> Apartment {
        if let Apartment::Created(inner) = &mut state {
            inner.rooms.push(
                crate::generated::domain::apartment::aggregate::ApartmentRoomsItem {
                    id: self.room_id,
                },
            );
        }
        state
    }
}
