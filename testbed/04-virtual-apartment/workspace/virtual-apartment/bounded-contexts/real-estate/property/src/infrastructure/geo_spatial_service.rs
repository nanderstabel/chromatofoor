/// Third-party mapping and location service for property coordinates.
use async_trait::async_trait;
#[async_trait]
pub trait GeoSpatialService: Send + Sync {}
