use std::net::{IpAddr, Ipv4Addr};
use std::{net::SocketAddr, time::Duration};

use bytes::Buf;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};
use serde::de;
use serde_derive::Deserialize;

use crate::error::metainfo::BencodeDeError;
use crate::error::tracker::TrackerError;

pub mod announce;
pub mod response;
mod test;
#[allow(clippy::module_inception)]
pub mod tracker;

pub mod prelude {
  pub use super::announce::*;
  pub use super::deserialize_peers;
  pub use super::deserialize_seconds;
  pub use super::response::*;
  pub use super::tracker::*;
  pub use crate::error::tracker::Result;
}

/// Deserialize an integer representing seconds into `Duration`.
pub fn deserialize_seconds<'de, D>(
  deserializer: D,
) -> Result<Option<Duration>, D::Error>
where
  D: de::Deserializer<'de>,
{
  let s: Option<u64> = de::Deserialize::deserialize(deserializer)?;
  Ok(s.map(Duration::from_secs))
}

/// Peers can be sent in two ways:
/// - as a bencode list of dicts including full peer metadata.
/// - as a single bencode string that contains only the peer Ip and Port in compact representation.
///
/// This method is to help to deserialize both into same type,
/// discarding the peer id present in full representation.
/// Cos of most of trackers send the compact response by default,
/// and here we do not use the peer id in the stage of
/// receiving a peer list from the tracker, so discarding is available.
pub fn deserialize_peers<'de, D>(
  deserializer: D,
) -> Result<Vec<SocketAddr>, D::Error>
where
  D: de::Deserializer<'de>,
{
  struct Visitor;

  impl<'de> de::Visitor<'de> for Visitor {
    type Value = Vec<SocketAddr>;
    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter.write_str("a string or list of dicts representing peer")
    }

    /// Deserializes a compact string of peers.
    ///
    /// Each entry is 6 bytes long, where the first 4 bytes are the IPv4 address,
    /// and then the last 2 bytes are the Port.
    ///
    /// Both are in network byte order.
    fn visit_bytes<E>(self, mut b: &[u8]) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      const ENTRY_LEN: usize = 6;

      let buf_len = b.len();

      if buf_len % ENTRY_LEN != 0 {
        return Err(TrackerError::BencodeDe(BencodeDeError::Message(
          "peers compact string must be a multiple of 6".into(),
        )))
        .map_err(E::custom);
      }

      let mut peers = Vec::with_capacity(buf_len / ENTRY_LEN);

      for _ in (0..buf_len).step_by(ENTRY_LEN) {
        let addr = Ipv4Addr::from(b.get_u32());
        let port = b.get_u16();
        let peer = SocketAddr::new(IpAddr::V4(addr), port);
        peers.push(peer);
      }
      Ok(peers)
    }

    /// Deserializes a list of dicts containing the peer information.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
      A: de::SeqAccess<'de>,
    {
      #[derive(Debug, Deserialize)]
      struct RawPeer {
        ip: String,
        port: u16,
      }
      let mut peers = Vec::with_capacity(seq.size_hint().unwrap_or(0));
      while let Some(RawPeer { ip, port }) = seq.next_element()? {
        let ip = if let Ok(ip) = ip.parse() {
          ip
        } else {
          continue;
        };
        peers.push(SocketAddr::new(ip, port));
      }

      Ok(peers)
    }
  }

  deserializer.deserialize_any(Visitor)
}

/// Contains the characters that need to be URL encoded according to:
/// https://en.wikipedia.org/wiki/Percent-encoding#Types_of_URI_characters
const URL_ENCODE_RESERVED: &AsciiSet = &NON_ALPHANUMERIC
  .remove(b'-')
  .remove(b'_')
  .remove(b'~')
  .remove(b'.');
