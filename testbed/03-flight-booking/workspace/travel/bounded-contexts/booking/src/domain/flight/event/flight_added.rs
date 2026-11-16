use crate::generated::domain::flight::aggregate::Flight;
use crate::generated::domain::flight::event::FlightAdded;
use chromatofoor::bounded_context::Event;

impl Event<Flight> for FlightAdded {
    fn evolve(&self, _state: Flight) -> Flight {
        Flight::Created(crate::generated::domain::flight::aggregate::FlightInner {
            flight_id: self.flight_id.clone(),
            reserved_seats: 0,
            total_seats: self.total_seats,
        })
    }
}
