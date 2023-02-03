use std::{fmt, sync::atomic::AtomicU32};

use crate::blockinfo::{BlockData, BlockInfo};

/// A SHA-1 hash digest, 20 bytes long.
pub type Sha1Hash = [u8; 20];

/// The peer ID is an arbitrary 20 byte string.
///
/// [`Guidelines for choosing a peer ID`](http://bittorrent.org/beps/bep_0020.html).
pub type PeerId = [u8; 20];

/// The bitfield represents the piece availability of a peer.
///
/// It is a compact bool vector of most significant bits to least
/// significant bits, that is, where the hightest bit represents the first piece,
/// the second highest element represents the second piece, and so on.
///
///  A truthy boolean value of a piece's position in this vector means
/// that peer has the piece, while a falsy value means that peer doesn't have
/// the piece.
pub type Bitfield = bitvec::prelude::BitVec<u8>;

/// This is the only block length we're dealing with (except for possibly the
/// last block).  It is the widely used and accepted 16 KiB.
pub const BLOCK_LEN: u32 = 0x4000;
// pub const BLOCK_LEN: u32 = 4;

/// The type of a piece's index.
///
/// On the wire all integers are sent as 4-byte big endian integers, but in the
/// source code we use `usize` to be consistent with other index types in Rust.
pub(crate) type PieceIndex = usize;

/// The type of a file's index.
pub(crate) type FileIndex = usize;

/// Each torrent gets a randomly assigned ID that is globally unique.
/// This id used in engine APIs to interact with torrents.
#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Hash,
)]
pub struct TorrentId(u32);

impl TorrentId {
    pub fn new() -> Self {
        static TORRENT_ID: AtomicU32 =
            AtomicU32::new(0);

        // the atomic is not synchronized data access around
        // it so relaxed ordering is fine for out purposes.
        let id = TORRENT_ID.fetch_add(
            1,
            std::sync::atomic::Ordering::Release,
        );
        TorrentId(id)
    }
}

impl fmt::Display for TorrentId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "t#{}", self.0)
    }
}

/// A piece block that contains the block's metadata and data.
pub struct Block {
    /// The index of the piece of which this is a block.
    pub piece_index: PieceIndex,
    /// The zero-based byte offset into the piece.
    pub offset: u32,
    /// The actual raw data of the block.
    pub data: BlockData,
}

impl Block {
    /// Constructs a new block based on the metadata and data.
    pub fn new(
        info: BlockInfo,
        data: impl Into<BlockData>,
    ) -> Self {
        Block {
            piece_index: info.piece_index,
            offset: info.offset,
            data: data.into(),
        }
    }

    /// Returns a [`BlockInfo`] representing the metadata of this
    /// block.
    pub fn info(&self) -> BlockInfo {
        BlockInfo {
            piece_index: self.piece_index,
            offset: self.offset,
            len: self.data.len() as u32,
        }
    }
}
