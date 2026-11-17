#[allow(unused_imports)]
use crate::generated::application::read_models::booking_status::{
    BookingStatus, BookingStatusRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-requested`
///
/// Rule: Insert booking-status record with payment-status Pending.
impl Project<crate::generated::domain::booking_payment::event::payment_requested::PaymentRequested>
    for BookingStatus
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::booking_payment::event::payment_requested::PaymentRequested,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::PaymentRequested::project for BookingStatus")
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-completed`
///
/// Rule: Update payment-status to Paid.
impl Project<crate::generated::domain::booking_payment::event::payment_completed::PaymentCompleted>
    for BookingStatus
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::booking_payment::event::payment_completed::PaymentCompleted,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::PaymentCompleted::project for BookingStatus")
    }
}
/// Read Model: `booking-status` — Materialized payment status view for travelers.
/// Projection: `payment-failed`
///
/// Rule: Update payment-status to Failed.
impl Project<crate::generated::domain::booking_payment::event::payment_failed::PaymentFailed>
    for BookingStatus
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::booking_payment::event::payment_failed::PaymentFailed,
        >,
    ) -> Result<(), ViewError> {
        todo!("implement Project::PaymentFailed::project for BookingStatus")
    }
}
