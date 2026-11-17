#[allow(unused_imports)]
use crate::generated::application::read_models::booking_status::{
    BookingStatus, BookingStatusRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
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
