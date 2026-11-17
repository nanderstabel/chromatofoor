use crate::generated::domain::cargo_booking::aggregate::CargoBooking;
use crate::generated::domain::cargo_booking::command::BookCargo;
use crate::generated::domain::cargo_booking::error::CargoBookingError;
use crate::generated::domain::cargo_booking::event::*;
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
        todo!("implement Command::CargoBooking::decide for BookCargo")
    }
}
