use crate::generated::domain::cargo_booking::aggregate::CargoBooking;
use crate::generated::domain::cargo_booking::event::CargoAssignedToRoute;
use chromatofoor::bounded_context::Event;
impl Event<CargoBooking> for CargoAssignedToRoute {
    fn evolve(&self, _state: CargoBooking) -> CargoBooking {
        todo!(
            "implement Event::CargoBooking::evolve for CargoAssignedToRoute (return _state if stateless)"
        )
    }
}
