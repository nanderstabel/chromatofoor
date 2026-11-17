use crate::generated::domain::voyage::{
    aggregate::{Voyage, VoyageInner},
    event::VoyageScheduled,
};
use chromatofoor::bounded_context::Event;
/// Event: `voyage-scheduled` — Emitted when a vessel voyage is scheduled.
impl Event<Voyage> for VoyageScheduled {
    fn evolve(&self, _state: Voyage) -> Voyage {
        Voyage::Created(VoyageInner {
            arrival_locode: self.arrival_locode.clone(),
            departure_locode: self.departure_locode.clone(),
            voyage_number: self.voyage_number.clone(),
        })
    }
}
