use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    blockinfo::BlockInfo,
    disk,
    error::disk::{ReadError, WriteError},
    peer::SessionTick,
    storage_info::StorageInfo,
    PeerId, PieceIndex, Sha1Hash, TorrentId, download::PieceDownload,
};

pub mod stats;

/// The channel for communication with torrent.
pub type Sender = UnboundedSender<Command>;

/// The type of channel on which a torrent can listen for
/// block write completion.
pub type Receiver = UnboundedSender<Command>;

/// The types of message that torrent can receive from parts of
/// the engine.
pub enum Command {
    /// Sent when some blocks were written to disk or an error occurred while
    /// writing.
    PieceCompletion(Result<PieceCompletion, WriteError>),

    /// There was an error reading a block.
    ReadError {
        block_info: BlockInfo,
        error: ReadError,
    },

    /// A message sent only once, after the peer has been connected.
    PeerConnected { addr: SocketAddr, id: PeerId },

    /// Peer sessions periodically send this message when they have a state change.
    PeerState { addr: SocketAddr, info: SessionTick },

    /// Graceful shutdown the torrent.
    ///
    /// This command tells all active peer sessions of torrent to do the same,
    /// waits for them and announce to trackers our exit.
    Shutdown,
}

/// The type returned on completing a piece.
#[derive(Debug)]
pub struct PieceCompletion {
    /// The index of the piece.
    pub index: PieceIndex,
    /// Whether the piece is valid. If it's not, it's not written to disk.
    pub is_valid: bool,
}

pub struct TorrentContext {
    pub id: TorrentId,
    pub info_hash: Sha1Hash,
    pub client_id: PeerId,
    pub cmd_tx: Sender,
    // pub piece_picker: Arc<RwLock<PiecePicker>>,
    pub download: RwLock<HashMap<PieceIndex, RwLock<PieceDownload>>>,
    // pub alert_tx: AlertSender,
    // pub disk_tx: disk::Sender,
    pub storage: StorageInfo,
}
