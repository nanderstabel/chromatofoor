use crate::generated::domain::cargo_booking::{
    aggregate::CargoBooking, command::BookCargo, error::CargoBookingError, event::*,
};
use chromatofoor::bounded_context::Command;
/// Command: `book-cargo` — Books a new cargo shipment between an origin port and a distinct destination port.
///
/// - Actors: `shipper`
/// - Publishes: `cargo-booked`
/// - Invariants (`CargoBookingError`):
///   - `same-origin-and-destination` — Origin UN/LOCODE and destination UN/LOCODE cannot be identical.
impl Command<CargoBooking> for BookCargo {
    type Error = CargoBookingError;
    #[tracing::instrument(skip(self, _state))]
    async fn decide(self, _state: &CargoBooking) -> Result<Vec<CargoBookingEvent>, Self::Error> {
        if self.origin_locode == self.destination_locode {
            return Err(CargoBookingError::SameOriginAndDestination);
        }
        Ok(vec![CargoBookingEvent::CargoBooked(CargoBooked {
            destination_locode: self.destination_locode,
            origin_locode: self.origin_locode,
            tracking_id: self.tracking_id,
        })])
    }
}
