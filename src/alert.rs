//! This module defines the alerts the API user may receive from torrent engine.
//!
//! Communication of such alerts is performed via unbounded [tokio::mpsc::channel].
//! Thus, the application should in which the engine is integrated may be driven
//! particular or entirely by this crate alerts.
//!
//! # Optional information
//!
//! By default, only the most basic alerts are broadcast from the engine.
//! The reason for this is that the crate follows a philosophy similar lies
//! behind Rust or Cpp.
//!
//! This is of course not fully possible with something as complex as a torrent
//! engine, but an effort is made to make more expensive operations optional.
//!
//! Such alerts include the
//! - [latest downloaded pieces]
//! - [peers]

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{error::Error, torrent::stats::TorrentStats, TorrentId};

pub type AlertSender = UnboundedSender<Alert>;
/// The channel on which alerts from the engine can be received ([`Alert`])
/// for the type fo message that can be received.
pub type AlertReceiver = UnboundedReceiver<Alert>;

/// The alerts that the engine may send the library user.
#[derive(Debug)]
#[non_exhaustive]
pub enum Alert {
  /// Posted when the torrent has finished downloading.
  TorrentComplete(TorrentId),
  /// Each running torrent sends an update of its latest statistics
  /// every second via this alert.
  TorrentStats {
    id: TorrentId,
    stats: Box<TorrentStats>,
  },
  /// An error from somewhere inside the engine.
  Error(Error),
}
