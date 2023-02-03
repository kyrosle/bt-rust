use std::io::{self, Cursor};

use bytes::{Buf, BufMut};
use tokio_util::codec::{Decoder, Encoder};

pub const PROTOCOL_STRING: &str =
    "BitTorrent protocol";
/// The message sent at the beginning of a peer session by both
/// sides of the connection.
///
/// handshake data format:
///
/// ```txt
/// <Protocol Identify length><Protocol Identify><Reversed><Info_hash> <Peer_id>
///
/// |   ---- 8 bytes ----    |-----19 bytes----|-8 bytes-|-20 bytes-|-20 bytes-|
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Handshake {
    /// The protocol string, which must equal "BitTorrent protocol",
    /// as otherwise the connection will aborted.
    pub prot: [u8; 19],
    /// A reserved field, currently all zero. This is where the client's
    /// supported extensions are announced.
    pub reserved: [u8; 8],
    /// The torrent's SHA1 info hash, used to identify the torrent in the
    /// handshake and to verify the peer.
    pub info_hash: [u8; 20],
    /// The arbitrary peer id, usually used to identify the torrent client.
    pub peer_id: [u8; 20],
}

impl Handshake {
    /// Create a new protocol version 1 handshake with the given info_hash and peer_id.
    pub fn new(
        info_hash: [u8; 20],
        peer_id: [u8; 20],
    ) -> Self {
        let mut prot = [0; 19];
        prot.copy_from_slice(
            PROTOCOL_STRING.as_bytes(),
        );
        Handshake {
            prot,
            reserved: [0; 8],
            info_hash,
            peer_id,
        }
    }
    /// Returns the length of handshake, in bytes.
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> u64 {
        19 + 8 + 20 + 20
    }
}

pub struct HandshakeCodec;

impl Encoder<Handshake> for HandshakeCodec {
    type Error = io::Error;
    fn encode(
        &mut self,
        handshake: Handshake,
        buf: &mut bytes::BytesMut,
    ) -> io::Result<()> {
        let Handshake {
            prot,
            reserved,
            info_hash,
            peer_id,
        } = handshake;

        // protocol length prefix
        debug_assert_eq!(prot.len(), 19);
        buf.put_u8(prot.len() as u8);
        // we should only be sending the bittorrent protocol string
        debug_assert_eq!(
            prot,
            PROTOCOL_STRING.as_bytes()
        );

        // payload
        buf.extend_from_slice(&prot);
        buf.extend_from_slice(&reserved);
        buf.extend_from_slice(&info_hash);
        buf.extend_from_slice(&peer_id);

        Ok(())
    }
}

impl Decoder for HandshakeCodec {
    type Item = Handshake;
    type Error = io::Error;

    fn decode(
        &mut self,
        buf: &mut bytes::BytesMut,
    ) -> io::Result<Option<Handshake>> {
        if buf.is_empty() {
            return Ok(None);
        }

        // `get_*` integer extractors consume the message bytes by advancing
        // buf's internal cursor. However, we don't want to do this as at this
        // point we aren't sure we have the full message in the buffer, and thus
        // we just want to peek at this value.
        let mut tmp_buf = Cursor::new(&buf);
        let prot_len = tmp_buf.get_u8() as usize;
        if prot_len != PROTOCOL_STRING.as_bytes().len()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                r#"Handshake must have the string "BitTorrent protocol"."#,
            ));
        }

        // check that we got the full payload in the buffer.
        // NOTE: we need to add the message length prefix's byte count
        // to msg_len since the buffer cursor was not advanced and thus
        // we need to consider the prefix too.
        let payload_len = prot_len + 8 + 20 + 20;
        if buf.remaining() > payload_len {
            // we have the full message in the buffer so advance the buffer
            // cursor past the message length header.
            buf.advance(1);
        } else {
            return Ok(None);
        }

        // protocol string
        let mut prot = [0; 19];
        buf.copy_to_slice(&mut prot);
        // reversed field
        let mut reserved = [0; 8];
        buf.copy_to_slice(&mut reserved);
        // info hash
        let mut info_hash = [0; 20];
        buf.copy_to_slice(&mut info_hash);
        // peer id
        let mut peer_id = [0; 20];
        buf.copy_to_slice(&mut peer_id);
        Ok(Some(Handshake {
            prot,
            reserved,
            info_hash,
            peer_id,
        }))
    }
}
