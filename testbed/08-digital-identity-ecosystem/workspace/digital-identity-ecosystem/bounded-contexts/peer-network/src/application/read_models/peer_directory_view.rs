#[allow(unused_imports)]
use crate::generated::application::read_models::peer_directory_view::{
    PeerDirectoryView, PeerDirectoryViewRecord,
};
use chromatofoor::{
    architecturekit::{Envelope, ViewError},
    bounded_context::Project,
};
impl Project<crate::generated::domain::peer_connection::event::peer_discovered::PeerDiscovered>
    for PeerDirectoryView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::peer_connection::event::peer_discovered::PeerDiscovered,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
impl Project<crate::generated::domain::peer_connection::event::peer_trusted::PeerTrusted>
    for PeerDirectoryView
{
    async fn project(
        &self,
        _event: Envelope<
            crate::generated::domain::peer_connection::event::peer_trusted::PeerTrusted,
        >,
    ) -> Result<(), ViewError> {
        Ok(())
    }
}
