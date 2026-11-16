/// An external API used for sending emails.
use async_trait::async_trait;
#[async_trait]
pub trait SendgridApi: Send + Sync {}
