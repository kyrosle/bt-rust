//! Set of module Error
pub mod disk;
pub mod metainfo;
pub mod peer;
pub mod torrent;
pub mod tracker;

use std::net::SocketAddr;

pub use disk::{NewTorrentError, ReadError, Result as DiskResult, WriteError};
pub use peer::{PeerError, Result as PeerResult};
pub use tokio::{io::Error as IoError, sync::mpsc::error::SendError};
pub use torrent::{Result as TorrentResult, TorrentError};
pub use tracker::{Result as TrackerResult, TrackerError};

use crate::TorrentId;

pub type EngineResult<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  #[error("channel error")]
  /// The cannel on which some component in engine was listening or sending died.
  Channel,

  #[error("invalid download path")]
  /// The torrent download location is not valid.
  InvalidDownloadPath,

  #[error("invalid torrent id")]
  /// The torrent ID did not correspond to any entry.
  /// This is returned when user specified a torrent that does not exist.
  InvalidTorrentId,

  #[error("{0}")]
  /// Holds global IO related errors.
  Io(IoError),

  #[error("torrent {id} error: {error}")]
  /// An error specific to a torrent
  Torrent { id: TorrentId, error: TorrentError },

  #[error("torrent {id} tracker error: {error}")]
  /// An error that occurred while a torrent was announcing to tracker.
  Tracker { id: TorrentId, error: TrackerError },

  #[error("torrent {id} peer {addr} error: {error}")]
  /// An error that occurred in a torrent's session with a peer.
  Peer {
    id: TorrentId,
    addr: SocketAddr,
    error: PeerError,
  },
}

impl From<IoError> for Error {
  fn from(value: IoError) -> Self {
    // the pieces field is a concatenation of 20 byte SHA-1 hashes, so it
    // must be a multiple of 20
    Self::Io(value)
  }
}

impl<T> From<SendError<T>> for Error {
  fn from(_: SendError<T>) -> Self {
    Self::Channel
  }
}
