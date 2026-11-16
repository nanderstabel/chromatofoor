use crate::generated::application::read_models::booking_details::BookingDetails;
use crate::generated::domain::booking::event::{
    booking_confirmed::BookingConfirmed, booking_failed::BookingFailed,
    booking_initialized::BookingInitialized,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<BookingInitialized> for BookingDetails {
    async fn project(&self, event: Envelope<BookingInitialized>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.booking_id = event.data.booking_id.to_string();
            rec.flight_id = Some(event.data.flight_id.to_string());
            rec.amount = Some(event.data.amount);
            rec.status = "Pending".to_string();
        })?;
        Ok(())
    }
}

impl Project<BookingConfirmed> for BookingDetails {
    async fn project(&self, event: Envelope<BookingConfirmed>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.status = "Confirmed".to_string();
        })?;
        Ok(())
    }
}

impl Project<BookingFailed> for BookingDetails {
    async fn project(&self, event: Envelope<BookingFailed>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.status = "Failed".to_string();
            rec.reason = Some(event.data.reason.clone());
        })?;
        Ok(())
    }
}
