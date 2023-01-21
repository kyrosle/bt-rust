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
pub type Bitfield = bitvec::prelude::BitVec;

/// This is the only block length we're dealing with (except for possibly the
/// last block).  It is the widely used and accepted 16 KiB.
pub(crate) const BLOCK_LEN: u32 = 0x4000;

/// The type of a piece's index.
///
/// On the wire all integers are sent as 4-byte big endian integers, but in the
/// source code we use `usize` to be consistent with other index types in Rust.
pub(crate) type PieceIndex = usize;
