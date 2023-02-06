use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
};

use tokio::{
    sync::mpsc::{
        self, UnboundedReceiver, UnboundedSender,
    },
    task,
};

use crate::{
    alert::AlertSender,
    conf::{Conf, TorrentConf},
    disk,
    error::{
        EngineResult, NewTorrentError, TorrentResult,
    },
    metainfo::Metainfo,
    storage_info::StorageInfo,
    torrent,
    tracker::tracker::Tracker,
    Bitfield, TorrentId,
};

/// The channel through which the user can send commands to the engine.
pub type Sender = UnboundedSender<Command>;
/// The channel on which the engine listens for commands from the user.
type Receiver = UnboundedReceiver<Command>;

/// The type of commands that the engine can receive.
pub enum Command {
    /// Contains the information for creating a new torrent.
    /// warning: the `TorrentParams` is too large, suggesting convert into Box<>
    CreateTorrent {
        id: TorrentId,
        params: Box<TorrentParams>,
    },
    /// Torrent allocation result. If successful, the id of the allocated
    /// torrent is returned for identification, if not, the reason of the
    /// error is included.
    TorrentAllocation {
        id: TorrentId,
        result: Result<(), NewTorrentError>,
    },
    /// Gracefully shuts down the engine and waits for all its torrents to do
    /// the same.
    Shutdown,
}

/// Information for creating a new torrent.
pub struct TorrentParams {
    /// Contains the torrent's metadata.
    pub metainfo: Metainfo,
    /// If set, overrides the default global config.
    pub conf: Option<TorrentConf>,
    /// Whether to download or seed the torrent.
    ///
    /// This is expected to be removed as this will become automatic once
    /// torrent resume data is supported.
    pub mode: Mode,
    /// The address on which the torrent should listen for new peers.
    pub listen_addr: Option<SocketAddr>,
}

/// The download mode.
/// TODO: remove in favor of automatic detection.
/// TODO: when seeding is specified, we need to verify that the files to be
/// seeded exist and are complete.
#[derive(Debug)]
pub enum Mode {
    Download { seeds: Vec<SocketAddr> },
    Seed,
}

impl Mode {
    fn own_pieces(
        &self,
        piece_count: usize,
    ) -> Bitfield {
        match self {
            Mode::Download { .. } => {
                Bitfield::repeat(false, piece_count)
            }
            Mode::Seed => {
                Bitfield::repeat(true, piece_count)
            }
        }
    }

    fn seeds(self) -> Vec<SocketAddr> {
        match self {
            Mode::Download { seeds } => seeds,
            _ => Vec::new(),
        }
    }
}

struct Engine {
    /// All currently running torrents in engine.
    torrents: HashMap<TorrentId, TorrentEntry>,

    /// The port on which other entities in the engine,
    /// or the API consumer sends the engine commands.
    cmd_rx: Receiver,

    /// the disk channel
    disk_tx: disk::Sender,
    disk_join_handle: Option<disk::JoinHandle>,

    /// The channel on which tasks in the engine post alerts to user.
    alert_tx: AlertSender,

    /// The global engine configuration that includes defaults for torrents
    /// whose config is not overridden.
    conf: Conf,
}

/// A running torrent's entry in the engine.
struct TorrentEntry {
    /// The torrent's command channel on which engine sends commands to torrent.
    tx: torrent::Sender,
    /// The torrent task's join handle, used during shutdown.
    join_handle:
        Option<task::JoinHandle<TorrentResult<()>>>,
}

impl Engine {
    /// Creates a new engine, spawning the disk task.
    fn new(
        conf: Conf,
        alert_tx: AlertSender,
    ) -> EngineResult<(Self, Sender)> {
        let (cmd_tx, cmd_rx) =
            mpsc::unbounded_channel();
        let (disk_join_handle, disk_tx) =
            disk::spawn(cmd_tx.clone())?;

        Ok((
            Engine {
                torrents: HashMap::new(),
                cmd_rx,
                disk_tx,
                disk_join_handle: Some(
                    disk_join_handle,
                ),
                alert_tx,
                conf,
            },
            cmd_tx,
        ))
    }

    async fn run(&mut self) -> EngineResult<()> {
        log::info!("Starting engine");

        while let Some(cmd) = self.cmd_rx.recv().await
        {
            match cmd {
                Command::CreateTorrent {
                    id,
                    params,
                } => todo!(),
                Command::TorrentAllocation {
                    id,
                    result,
                } => todo!(),
                Command::Shutdown => todo!(),
            }
        }

        Ok(())
    }

    async fn create_torrent(
        &mut self,
        id: TorrentId,
        params: TorrentParams,
    ) -> EngineResult<()> {
        let conf = params.conf.unwrap_or_else(|| {
            self.conf.torrent.clone()
        });
        let storage_info = StorageInfo::new(
            &params.metainfo,
            self.conf.engine.download_dir.clone(),
        );

        let trackers = params
            .metainfo
            .trackers
            .into_iter()
            .map(Tracker::new)
            .collect::<Vec<_>>();

        let own_pieces = params
            .mode
            .own_pieces(storage_info.piece_count);

        // let (mut torrent, torrent_tx) =
        //     Torrent::new(torrent::Params {
        //         id,
        //         disk_tx: self.disk_tx.clone(),
        //         info_hash: params.metainfo.info_hash,
        //         storage_info: storage_info.clone(),
        //         own_pieces,
        //         trackers,
        //         client_id: self.conf.engine.client_id,
        //         listen_addr: params
        //             .listen_addr
        //             .unwrap_or_else(|| {
        //                 SocketAddr::new(
        //                     Ipv4Addr::UNSPECIFIED
        //                         .into(),
        //                     0,
        //                 )
        //             }),
        //         conf,
        //         alert_tx: self.alert_tx.clone(),
        //     });
        Ok(())
    }
}
