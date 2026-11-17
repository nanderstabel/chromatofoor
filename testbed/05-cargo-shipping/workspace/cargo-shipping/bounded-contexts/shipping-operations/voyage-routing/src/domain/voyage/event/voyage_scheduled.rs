use crate::generated::domain::voyage::aggregate::Voyage;
use crate::generated::domain::voyage::event::VoyageScheduled;
use chromatofoor::bounded_context::Event;
/// Event: `voyage-scheduled` — Emitted when a vessel voyage is scheduled.
impl Event<Voyage> for VoyageScheduled {
    fn evolve(&self, _state: Voyage) -> Voyage {
        todo!("implement Event::Voyage::evolve for VoyageScheduled (return _state if stateless)")
    }
}
