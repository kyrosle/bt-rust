use std::{collections::HashMap, sync::Arc};

use crate::{storage_info::StorageInfo, PieceIndex, torrent};

use super::piece::Piece;

/// Torrent information related to disk IO.
/// 
/// Contains the in-progress pieces (i.e. the writer buffer), metadata about
/// torrent's download and piece sizes, etc.
#[allow(dead_code)]
pub struct Torrent {
    /// All information concerning the this torrent's storage.
    info: StorageInfo,

    /// The in-progress piece downloads and disk writes. This is the torrent's
    /// disk write buffer. Each piece is mapped to its index for faster lookups.
    write_buf: HashMap<PieceIndex, Piece>,

    /// Contains the fields that may be accessed by other threads.
    /// 
    /// This is an optimization to avoid having to call
    /// `Arc::clone(&self.fields)` for each of the contained fields when sending
    /// them to an IO worker threads.
    thread_ctx: Arc<ThreadContext>,

    /// The concatenation of all expected piece hashes.
    piece_hashes: Vec<u8>,
}

/// Contains fields that are commonly accessed by torrent's IO threads.
/// 
/// We're using blocking IO to read things from disk and so such operations need to be
/// disk task.
/// 
/// But these threads need some fields from torrent and so those fields 
/// would need to be in an arc each. With this optimization, only this struct needs to
/// be in an arc and thus only a single atomic increment has to
/// be made when sending the contains fields across threads.
struct ThreadContext {
}
