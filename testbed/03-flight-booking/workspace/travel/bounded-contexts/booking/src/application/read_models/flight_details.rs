use crate::generated::application::read_models::flight_details::FlightDetails;
use crate::generated::domain::flight::event::{
    flight_added::FlightAdded, seat_released::SeatReleased, seat_reserved::SeatReserved,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<FlightAdded> for FlightDetails {
    async fn project(&self, event: Envelope<FlightAdded>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.flight_id = event.data.flight_id.to_string();
            rec.total_seats = event.data.total_seats;
            rec.available_seats = event.data.total_seats;
        })?;
        Ok(())
    }
}

impl Project<SeatReserved> for FlightDetails {
    async fn project(&self, event: Envelope<SeatReserved>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.available_seats -= 1;
            rec.reserved_seats += 1;
        })?;
        Ok(())
    }
}

impl Project<SeatReleased> for FlightDetails {
    async fn project(&self, event: Envelope<SeatReleased>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.available_seats += 1;
            rec.reserved_seats -= 1;
        })?;
        Ok(())
    }
}
