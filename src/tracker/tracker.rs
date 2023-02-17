use std::fmt;

use reqwest::{Client, Url};

use super::prelude::Result;
use super::URL_ENCODE_RESERVED;
use super::{announce::Announce, response::Response};

/// The HTTP tracker for a tonnert for which we can request peers as well as to announce transfer progress.
pub struct Tracker {
  /// The HTTP client (from reqwest::Client)
  client: Client,
  /// The URL of the tracker.
  url: Url,
}

impl Tracker {
  pub fn new(url: Url) -> Self {
    Tracker {
      client: Client::new(),
      url,
    }
  }

  /// Sends an announce request to the tracker with the specified parameters.
  ///
  /// This may be used by a torrent to request peers to download form.
  /// And report the current status information to the the tracker.
  pub async fn announce(&self, params: Announce) -> Result<Response> {
    let mut query = vec![
      ("port", params.port.to_string()),
      ("downloaded", params.downloaded.to_string()),
      ("uploaded", params.uploaded.to_string()),
      ("left", params.left.to_string()),
      ("compact", "1".to_string()),
    ];

    if let Some(peer_count) = params.peer_count {
      query.push(("numwant", peer_count.to_string()));
    }
    if let Some(ip) = &params.ip {
      query.push(("ip", ip.to_string()));
    }

    let url = format!(
      "{url}\
            ?peer_id={peer_id}\
            &info_hash={info_hash}",
      url = self.url,
      info_hash = percent_encoding::percent_encode(
        &params.info_hash,
        URL_ENCODE_RESERVED
      ),
      peer_id =
        percent_encoding::percent_encode(&params.peer_id, URL_ENCODE_RESERVED)
    );

    let resp = self
      .client
      .get(&url)
      .query(&query)
      .send()
      .await?
      .error_for_status()?
      .bytes()
      .await?;

    let resp = serde_bencode::from_bytes(&resp)?;
    Ok(resp)
  }
}

impl fmt::Display for Tracker {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "'{}'", self.url)
  }
}
