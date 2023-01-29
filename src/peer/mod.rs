use crate::counter::ThruputCounters;

use self::session::SessionState;

pub mod codec;
pub mod session;

/// The most essential information of a peer session 
/// that is sent to torrent with each session tick.
pub struct SessionTick {
    /// A snapshot of the session state.
    pub state: SessionState,
    /// Various transfer statistics.
    pub counters: ThruputCounters,
    /// The number of pieces the peer has available.
    pub piece_count: usize,
}
