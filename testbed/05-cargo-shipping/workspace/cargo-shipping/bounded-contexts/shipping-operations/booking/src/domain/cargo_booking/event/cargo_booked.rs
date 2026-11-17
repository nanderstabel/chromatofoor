use crate::generated::domain::cargo_booking::aggregate::CargoBooking;
use crate::generated::domain::cargo_booking::event::CargoBooked;
use chromatofoor::bounded_context::Event;
impl Event<CargoBooking> for CargoBooked {
    fn evolve(&self, _state: CargoBooking) -> CargoBooking {
        todo!("implement Event::CargoBooking::evolve for CargoBooked (return _state if stateless)")
    }
}
