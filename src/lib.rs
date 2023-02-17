pub mod blockinfo;
pub mod disk;
pub mod download;
pub mod error;
pub mod metainfo;
pub mod peer;
pub mod piece_picker;
pub mod storage_info;
pub mod torrent;
pub mod tracker;

pub mod iovecs;

pub mod alert;
pub mod avg;
pub mod counter;

pub mod conf;
pub mod engine;

mod define;
pub use define::*;

pub mod prelude {
  pub use crate::{
    alert::{Alert, AlertReceiver},
    conf::Conf,
    engine::{
      self, EngineHandle, Mode, TorrentParams,
    },
    error::Error,
    metainfo::Metainfo,
    TorrentId,
  };
  pub use futures::stream::StreamExt;
}
