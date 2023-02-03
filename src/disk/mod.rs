use crate::{
    blockinfo::BlockInfo, error::*, peer,
    storage_info::StorageInfo, torrent, TorrentId,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task,
};

pub mod io;

pub fn spawn() {}

pub type JoinHandle = task::JoinHandle<DiskResult<()>>;

/// The channel for sending commands to the disk task.
pub type Sender = UnboundedSender<Command>;
/// The channel for the disk task uses to listen for commands.
pub type Receiver = UnboundedReceiver<Command>;

/// The type of commands that the disk can execute.
#[derive(Debug)]
pub enum Command {
    /// Allocate a new torrent in `Disk`.
    NewTorrent {
        id: TorrentId,
        storage_info: StorageInfo,
        piece_hashes: Vec<u8>,
        torrent_tx: torrent::Sender,
    },
    /// Request to eventually write a block to disk.
    WriteBlock {
        id: TorrentId,
        block_info: BlockInfo,
        data: Vec<u8>,
    },
    /// Request to eventually read a block from disk and return it via the
    /// sender.
    ReadBlock {
        id: TorrentId,
        block_info: BlockInfo,
        result_tx: peer::Sender,
    },
    /// Eventually shutdown the disk task.
    Shutdown,
}
