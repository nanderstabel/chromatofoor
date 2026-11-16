use crate::generated::application::read_models::payment_details::PaymentDetails;
use crate::generated::domain::payment::event::{
    payment_failed::PaymentFailed, payment_initiated::PaymentInitiated,
    payment_processed::PaymentProcessed,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};

impl Project<PaymentInitiated> for PaymentDetails {
    async fn project(&self, event: Envelope<PaymentInitiated>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.booking_id = event.data.booking_id.to_string();
            rec.amount = event.data.amount;
            rec.status = "Initiated".to_string();
        })?;
        Ok(())
    }
}

impl Project<PaymentProcessed> for PaymentDetails {
    async fn project(&self, event: Envelope<PaymentProcessed>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.booking_id = event.data.booking_id.to_string();
            rec.amount = event.data.amount;
            rec.status = "Processed".to_string();
        })?;
        Ok(())
    }
}

impl Project<PaymentFailed> for PaymentDetails {
    async fn project(&self, event: Envelope<PaymentFailed>) -> Result<(), ViewError> {
        self.records.upsert_subject(&event, |rec| {
            rec.booking_id = event.data.booking_id.to_string();
            rec.status = "Failed".to_string();
            rec.reason = Some(event.data.reason.clone());
        })?;
        Ok(())
    }
}
