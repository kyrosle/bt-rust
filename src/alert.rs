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

use crate::{error::Error, TorrentId};
pub enum Alert {
    TorrentComplete(TorrentId),
    TorrentStats {
        id: TorrentId,
        // stats: Box<TorrentStats>,
    },
    Error(Error),
}
