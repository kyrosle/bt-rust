use crate::{
    blockinfo::{BlockData, BlockInfo},
    Bitfield,
};

/// The actual message exchanged by peer.
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub enum Message {
    KeepAlive,
    BitField(Bitfield),
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        piece_index: usize,
    },
    Request(BlockInfo),
    Block {
        piece_index: usize,
        offset: u32,
        data: BlockData,
    },
    Cancel(BlockInfo),
}
