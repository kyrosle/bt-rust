use std::{
    collections::HashMap, net::SocketAddr, sync::Arc,
};

use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    RwLock,
};

use crate::{
    alert::AlertSender,
    blockinfo::BlockInfo,
    disk,
    download::PieceDownload,
    error::disk::{ReadError, WriteError},
    peer::SessionTick,
    piece_picker::PiecePicker,
    storage_info::StorageInfo,
    tracker::tracker::Tracker,
    Bitfield, PeerId, PieceIndex, Sha1Hash, TorrentId, conf::TorrentConf,
};

pub mod stats;

/// The channel for communication with torrent.
pub type Sender = UnboundedSender<Command>;

/// The type of channel on which a torrent can listen for
/// block write completion.
pub type Receiver = UnboundedReceiver<Command>;

/// The types of message that torrent can receive from parts of
/// the engine.
pub enum Command {
    /// Sent when some blocks were written to disk or an error occurred while
    /// writing.
    PieceCompletion(
        Result<PieceCompletion, WriteError>,
    ),

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

/// Information and methods shared with peer sessions in the torrent.
///
/// This type contains fields that need to be read or updated by peer sessions.
/// Fields expected to be mutated are thus secured for inter-task access with
/// various synchronization primitives.
pub struct TorrentContext {
    /// The torrent ID, unique in this engine.
    pub id: TorrentId,
    /// The info hash of the torrent, derived from its metainfo.
    /// This is used to identify the torrent with other peers and trackers.
    pub info_hash: Sha1Hash,
    /// The arbitrary client id, chosen by the user of this library.
    /// This is advertised to peers and trackers.
    pub client_id: PeerId,

    /// A copy of the torrent channel sender. This is not used by torrent itself,
    /// but by the peer session tasks to which an arc copy of this torrent
    /// context is given.
    pub cmd_tx: Sender,

    /// The piece picker picks the next most optimal piece to download and
    /// is shared by all peers in a torrent.
    pub piece_picker: Arc<RwLock<PiecePicker>>,
    /// These are the active piece downloads in which the peer sessions in this
    /// torrent are participating.
    ///
    /// They are stored and synchronized in this object to download a piece from
    /// multiple peers, which helps us to have fewer incomplete pieces.
    ///
    /// Peer sessions may be run on different threads, any of which may read and
    /// write to this map and to the pieces in the map. Thus we need to read
    /// write lock on both.
    pub download: RwLock<
        HashMap<PieceIndex, RwLock<PieceDownload>>,
    >,

    /// The channel on which to post alerts to user.
    pub alert_tx: AlertSender,

    /// The handle to the disk IO task, used to issue commands on it.
    /// A copy of this handle is passed down to each peer session.
    pub disk_tx: disk::Sender,

    /// Info about the torrent's storage (piece length, download length, etc).
    pub storage: StorageInfo,
}

/// Parameters for the torrent constructor.
pub struct Params {
    pub id: TorrentId,
    pub disk_tx: disk::Sender,
    pub info_hash: Sha1Hash,
    pub storage_info: StorageInfo,
    pub own_pieces: Bitfield,
    pub trackers: Vec<Tracker>,
    pub client_id: PeerId,
    pub listen_addr: SocketAddr,
    pub conf: TorrentConf,
    pub alert_tx: AlertSender,
}
