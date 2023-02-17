use std::{net::SocketAddr, time::Duration};

use serde_derive::Deserialize;

use super::{deserialize_peers, deserialize_seconds};

#[derive(Debug, Deserialize)]
#[cfg_attr(
  test,
  derive(PartialEq, serde_derive::Serialize)
)]
pub struct Response {
  /// The tracker Id. If set, we must send it with each subsequent announce.
  #[serde(rename = "tracker id")]
  pub tracker_id: Option<String>,

  /// If this is not empty, no other fields in response are valid.
  /// It contains a human-readable error message as to why the request was invalid.
  #[serde(rename = "failure reason")]
  pub failure_reason: Option<String>,

  /// Optional. Similar to failure_reason, but the response is still processed.
  pub warning_message: Option<String>,

  /// The number of seconds the client should wait before recontacting tracker.
  #[serde(deserialize_with = "deserialize_seconds")]
  pub interval: Option<Duration>,

  /// If present, the client must not re-announce it self before the end of this interval.
  #[serde(default)]
  #[serde(rename = "min interval")]
  #[serde(deserialize_with = "deserialize_seconds")]
  pub min_interval: Option<Duration>,

  #[serde(rename = "complete")]
  pub seeder_count: Option<usize>,
  #[serde(rename = "incomplete")]
  pub leecher_count: Option<usize>,

  #[serde(default)]
  #[serde(deserialize_with = "deserialize_peers")]
  pub peers: Vec<SocketAddr>,
}
