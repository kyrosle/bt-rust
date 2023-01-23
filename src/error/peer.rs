pub use tokio::{io::Error as IoError, sync::mpsc::error::SendError};

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("received unexpected bitfield")]
    /// The bitfield message was not sent after the handshake.
    /// According to the protocol,
    /// it should only be accepted after the handshake
    /// and when received at any other time, connection is severed.
    BitfieldNotAfterHandshake,

    #[error("channel error")]
    /// The channel on which some component in engine
    /// was listening or sending  died.
    Channel,

    #[error("chocked peer sent request")]
    /// Peers are not allowed to request blocks while they are chocked. If they do so, their connection is severed.
    RequestWhileChocked,

    #[error("inactivity timeout")]
    /// A peer session timed out because neither side of the
    /// connection became interested in each other.
    InactivityTimeout,

    #[error("invalid block info")]
    /// The block information the peer sent is invalid.
    InvalidBlockInfo,

    #[error("invalid piece index")]
    /// The block's piece index is invalid.
    InvalidPieceIndex,

    #[error("invalid info hash")]
    /// Peer's torrent info hash did not match ours.
    InvalidInfoHash,

    #[error("{0}")]
    /// An IO error occurred.
    Io(std::io::Error),
}

impl From<IoError> for PeerError {
    fn from(value: IoError) -> Self {
        // the piece field is a concatenation of 20 byte SHA-1 hashes,
        // so it must be a multiple of 20 bytes
        Self::Io(value)
    }
}

impl<T> From<SendError<T>> for PeerError {
    fn from(_: SendError<T>) -> Self {
        Self::Channel
    }
}
