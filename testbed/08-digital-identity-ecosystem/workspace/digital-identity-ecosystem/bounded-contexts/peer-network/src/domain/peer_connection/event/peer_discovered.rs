use crate::generated::domain::peer_connection::aggregate::PeerConnection;
use crate::generated::domain::peer_connection::event::PeerDiscovered;
use chromatofoor::bounded_context::Event;

impl Event<PeerConnection> for PeerDiscovered {
    fn evolve(&self, state: PeerConnection) -> PeerConnection {
        // TODO: Implement event evolution logic
        state
    }
}
