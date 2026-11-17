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
/// Rule: Insert booking-status record with payment-status Pending.
impl Project<PaymentRequested> for BookingStatus {
    async fn project(&self, _event: Envelope<PaymentRequested>) -> Result<(), ViewError> {
        todo!("implement Project::PaymentRequested::project for BookingStatus")
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-completed`
///
/// Rule: Update payment-status to Paid.
impl Project<PaymentCompleted> for BookingStatus {
    async fn project(&self, _event: Envelope<PaymentCompleted>) -> Result<(), ViewError> {
        todo!("implement Project::PaymentCompleted::project for BookingStatus")
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-failed`
///
/// Rule: Update payment-status to Failed.
impl Project<PaymentFailed> for BookingStatus {
    async fn project(&self, _event: Envelope<PaymentFailed>) -> Result<(), ViewError> {
        todo!("implement Project::PaymentFailed::project for BookingStatus")
    }
}
