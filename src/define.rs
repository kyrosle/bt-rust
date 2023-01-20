/// A SHA-1 hash digest, 20 bytes long.
pub type Sha1Hash = [u8;20];

/// The peer ID is an arbitrary 20 byte string.
///
/// [`Guidelines for choosing a peer ID`](http://bittorrent.org/beps/bep_0020.html).
pub type PeerId = [u8;20];