use tokio::io::Error as IoError;
use tokio::sync::mpsc::error::SendError;

pub type Result<T, E = TorrentError> =
    std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum TorrentError {
    #[error("channel error")]
    /// The channel on which some component in engine was
    /// listening or sending died.
    Channel,

    #[error("{0}")]
    /// An Io error occurred.
    Io(std::io::Error),
}

impl From<IoError> for TorrentError {
    fn from(value: IoError) -> Self {
        // the pieces field is a concatenation of 20 byte
        // SHA-1 hashes, so it must be a multiple of 20.
        Self::Io(value)
    }
}

impl<T> From<SendError<T>> for TorrentError {
    fn from(_: SendError<T>) -> Self {
        Self::Channel
    }
}
