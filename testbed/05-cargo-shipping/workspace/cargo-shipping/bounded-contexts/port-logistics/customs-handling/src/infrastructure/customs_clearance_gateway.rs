use async_trait::async_trait;
/// External national customs authority API receiving electronic cargo manifests.
#[async_trait]
pub trait CustomsClearanceGateway: Send + Sync {}
