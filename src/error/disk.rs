use crate::error::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error type returned on failed torrent allocations.
///
/// This error is non-fatal, so it should not be grouped with the
/// global `Error` type as it may be recovered from.
#[derive(Debug, thiserror::Error)]
pub enum NewTorrentError {
    #[error("disk torrent entry already exists")]
    /// The torrent entry already exists in `Disk`'s hashmap of torrents.
    AlreadyExists,
    #[error("{0}")]
    /// IO error while allocating torrent.
    Io(std::io::Error),
}

impl From<std::io::Error> for NewTorrentError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Error type returned on failed block writes.
///
/// This error is non-fatal so it should not be grouped with the global `Error`
/// type as it may be recovered from.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("{0}")]
    /// An IO error ocurred.
    Io(std::io::Error),
}

/// Error type returned on failed block reads.
///
/// This error is non-fatal so it should not be grouped with the global `Error`
/// type as it may be recovered from.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("invalid block offset")]
    /// The block's offset in piece is invalid.
    InvalidBlockOffset,

    #[error("torrent data missing")]
    /// The block is valid within torrent but its data has not been downloaded
    /// yet or has been deleted.
    MissingData,

    #[error("{0}")]
    /// An IO error occurred.
    Io(std::io::Error),
}
