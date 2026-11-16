use crate::generated::domain::peer_connection::aggregate::PeerConnection;
use crate::generated::domain::peer_connection::command::DiscoverPeer;
use crate::generated::domain::peer_connection::error::PeerConnectionError;
use crate::generated::domain::peer_connection::event::*;
use chromatofoor::bounded_context::Command;

impl Command<PeerConnection> for DiscoverPeer {
    type Error = PeerConnectionError;
    #[tracing::instrument(skip(self, _aggregate), err)]
    async fn decide(
        self,
        _aggregate: &PeerConnection,
    ) -> Result<Vec<PeerConnectionEvent>, Self::Error> {
        Ok(vec![])
    }
}
