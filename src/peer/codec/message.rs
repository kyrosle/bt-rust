use std::io;

use bytes::{BytesMut, BufMut};

use crate::{
    blockinfo::{BlockData, BlockInfo},
    Bitfield,
};

/// The ID of a message, which is included as a prefix in most messages.
///
/// The handshake and keep alive messages don't have explicit IDs.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MessageId {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Block = 7,
    Cancel = 8,
}

impl MessageId {
    /// Returns the header length of the specific message type.
    ///
    /// Since this is fix size for all messages, it can be determined simply from
    /// the message id.
    #[inline(always)]
    pub fn header_len(&self) -> u64 {
        match self {
            MessageId::Choke => 4 + 1,
            MessageId::Unchoke => 4 + 1,
            MessageId::Interested => 4 + 1,
            MessageId::NotInterested => 4 + 1,
            MessageId::Have => 4 + 1 + 4,
            MessageId::Bitfield => 4 + 1,
            MessageId::Request => 4 + 1 + 3 * 4,
            MessageId::Block => 4 + 1 + 2 * 4,
            MessageId::Cancel => 4 + 1 + 3 * 4,
        }
    }
}

impl TryFrom<u8> for MessageId {
    type Error = io::Error;
    fn try_from(k: u8) -> Result<Self, Self::Error> {
        use MessageId::*;
        match k {
            k if k == Choke as u8 => Ok(Choke),
            k if k == Unchoke as u8 => Ok(Unchoke),
            k if k == Interested as u8 => Ok(Interested),
            k if k == NotInterested as u8 => Ok(NotInterested),
            k if k == Have as u8 => Ok(Have),
            k if k == Bitfield as u8 => Ok(Bitfield),
            k if k == Request as u8 => Ok(Request),
            k if k == Block as u8 => Ok(Block),
            k if k == Cancel as u8 => Ok(Cancel),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unknown message id",
            )),
        }
    }
}

/// The actual message exchanged by peer.
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub enum Message {
    KeepAlive,
    Bitfield(Bitfield),
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

impl Message {
    /// Returns the ID of the message, if it has one  (e.g. keep alive doesn't).
    pub fn id(&self) -> Option<MessageId> {
        match self {
            Message::KeepAlive => None,
            Message::Bitfield(_) => Some(MessageId::Bitfield),
            Message::Choke => Some(MessageId::Choke),
            Message::Unchoke => Some(MessageId::Unchoke),
            Message::Interested => Some(MessageId::Interested),
            Message::NotInterested => Some(MessageId::NotInterested),
            Message::Have { .. } => Some(MessageId::Have),
            Message::Request(_) => Some(MessageId::Request),
            Message::Block { .. } => Some(MessageId::Block),
            Message::Cancel(_) => Some(MessageId::Cancel),
        }
    }

    /// Returns the length of the part of the message that constitutes the
    /// message header. For all but the block message this is simply the size of
    /// the message. For the block message this is the message header.
    pub fn protocol_len(&self) -> u64 {
        if let Some(id) = self.id() {
            id.header_len()
        } else {
            assert_eq!(*self, Self::KeepAlive);
            1
        }
    }
}

impl BlockInfo {
    /// Encode the block info in the network binary protocol's format
    /// into the given buffer.
    pub fn encode(&self, buf: &mut BytesMut) -> io::Result<()> {
        let piece_index = self
            .piece_index
            .try_into()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        buf.put_u32(piece_index);
        buf.put_u32(self.offset);
        buf.put_u32(self.len);
        Ok(())
    }
}

