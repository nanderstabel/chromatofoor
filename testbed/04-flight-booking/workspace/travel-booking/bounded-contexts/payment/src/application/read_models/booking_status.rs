#[allow(unused_imports)]
use crate::generated::application::read_models::booking_status::{
    BookingStatus, BookingStatusRecord,
};
use crate::generated::domain::booking_payment::event::PaymentCompleted;
use crate::generated::domain::booking_payment::event::PaymentFailed;
use crate::generated::domain::booking_payment::event::PaymentRequested;
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-requested`
///
/// Rule: Insert booking-status record keyed by booking-id with payment-status Pending.
impl Project<PaymentRequested> for BookingStatus {
    async fn project(&self, event: Envelope<PaymentRequested>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.booking_id.as_str(), &event, |record| {
                record.booking_id = event.data.booking_id.to_string();
                record.payment_status = "Pending".to_owned();
            })?;
        Ok(())
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-completed`
///
/// Rule: Update payment-status to Paid for the record keyed by booking-id.
impl Project<PaymentCompleted> for BookingStatus {
    async fn project(&self, event: Envelope<PaymentCompleted>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.booking_id.as_str(), &event, |record| {
                record.booking_id = event.data.booking_id.to_string();
                record.payment_status = "Paid".to_owned();
            })?;
        Ok(())
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-failed`
///
/// Rule: Update payment-status to Failed for the record keyed by booking-id.
impl Project<PaymentFailed> for BookingStatus {
    async fn project(&self, event: Envelope<PaymentFailed>) -> Result<(), ViewError> {
        self.records
            .upsert(event.data.booking_id.as_str(), &event, |record| {
                record.booking_id = event.data.booking_id.to_string();
                record.payment_status = "Failed".to_owned();
            })?;
        Ok(())
    }
}
