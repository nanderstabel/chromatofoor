/// External service for processing tenant lease payments.
use async_trait::async_trait;
#[async_trait]
pub trait PaymentGateway: Send + Sync {}
