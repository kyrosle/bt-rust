//! This module defines types used to configure the engine and its parts.

use std::{path::PathBuf, time::Duration};

use crate::PeerId;

pub const CLIENT_ID: &PeerId = b"cbt-0000000000000000";
// pub const CLIENT_ID: &PeerId = b"-qB1450-352885928458";
// pub static CLIENT_ID: Lazy<PeerId> = Lazy::new(|| {
//     let mut id = [0u8; 20];
//     let rid = get_random_string(20);
//     let rid = rid.as_bytes();
//     id[..].copy_from_slice(&rid[..20]);
//     id
// });

/// The global configuration for the torrent engine and all its parts.
#[derive(Debug, Clone)]
pub struct Conf {
  pub engine: EngineConf,
  pub torrent: TorrentConf,
}

// fn get_random_string(len: usize) -> String {
//   rand::thread_rng()
//     .sample_iter::<char, _>(rand::distributions::Standard)
//     .take(len)
//     .collect()
// }

impl Conf {
  /// Returns the torrent configuration with reasonable defaults,
  /// expected for the download directory, as it is not sensible
  /// to guess that for the user. It uses the default client id
  /// [`CLIENT_ID`]
  pub fn new(download_dir: impl Into<PathBuf>) -> Self {
    Self {
      engine: EngineConf {
        client_id: *CLIENT_ID,
        download_dir: download_dir.into(),
      },
      torrent: TorrentConf::default(),
    }
  }
}

/// Configuration related to the engine itself.
#[derive(Debug, Clone)]
pub struct EngineConf {
  /// The ID of the client to announce to trackers and other peers.
  pub client_id: PeerId,
  /// The directory in which a torrent's files are placed upon download and
  /// from which they are seeded.
  pub download_dir: PathBuf,
}

/// Configuration for a torrent
///
/// The engine will have a default instance of this applied to all torrents
/// by default, but individual torrents may override this configuration.
#[derive(Debug, Clone)]
pub struct TorrentConf {
  /// The minimum number of peers we want to keep in torrent at all times.
  /// This will be configurable later.
  pub min_requested_peer_count: usize,

  /// The max number of connected peers the torrent should have.
  pub max_connected_peer_count: usize,

  /// If the tracer doesn't provide a minimum announce interval, we default
  /// to announcing every 30 seconds.
  pub announce_interval: Duration,

  /// After this many attempts, the torrent stops announcing to a tracker.
  pub tracker_error_threshold: usize,

  /// Specifies which optional alerts to send, besides the default periodic
  /// stats update.
  pub alerts: TorrentAlertConf,
}

/// Configuration of a torrent's optional alerts.
///
/// By default, all optional alerts are turned off. This is because some of
/// these alerts may have overhead that shouldn't be paid when the alerts are
/// not used.
#[derive(Debug, Clone, Default)]
pub struct TorrentAlertConf {
  /// Receive the pieces that were completed each round.
  ///
  /// This has minor overhead and so it may be enabled. For full optimization,
  /// however, it is only enabled when either the pieces or individual file
  /// completions are needed.
  pub completed_pieces: bool,

  /// Receive aggregate statistics about the torrent's peers.
  ///
  /// This may be relatively expensive. It is suggested to only turn it on
  /// when it is specifically needed, e.g. when the UI is showing the peers of
  /// a torrent.
  pub peers: bool,
}

impl Default for TorrentConf {
  fn default() -> Self {
    TorrentConf {
      // We always request at least 10 peers as anything less is a waste
      // of network round trip and it allows us to buffer up a bit more
      // than needed.
      min_requested_peer_count: 10,
      // This value is mostly picked for performance while keeping in mind
      // not to overwhelm the host.
      max_connected_peer_count: 50,
      // need testing
      announce_interval: Duration::from_secs(60 * 60),
      // need testing
      tracker_error_threshold: 15,
      alerts: Default::default(),
    }
  }
}
