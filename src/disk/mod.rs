use std::collections::HashMap;

use crate::{
  blockinfo::BlockInfo, engine, error::*, peer, storage_info::StorageInfo,
  torrent, TorrentId,
};
use tokio::{
  sync::{
    mpsc::{self, UnboundedReceiver, UnboundedSender},
    RwLock,
  },
  task,
};

use self::io::torrent::Torrent;

pub mod io;

/// Spawns a disk IO task and returns a tuple with the task join handle
/// and the disk handle used for sending commands.
pub fn spawn(engine_tx: engine::Sender) -> EngineResult<(JoinHandle, Sender)> {
  log::info!("Spawning disk IO task");
  let (mut disk, dist_tx) = Disk::new(engine_tx)?;
  let join_handle = task::spawn(async move { disk.start().await });
  log::info!("Spawned disk IO task");

  Ok((join_handle, dist_tx))
}

pub type JoinHandle = task::JoinHandle<DiskResult<()>>;

/// The channel for sending commands to the disk task.
pub type Sender = UnboundedSender<Command>;
/// The channel for the disk task uses to listen for commands.
type Receiver = UnboundedReceiver<Command>;

/// The type of commands that the disk can execute.
#[derive(Debug)]
pub enum Command {
  /// Allocate a new torrent in `Disk`.
  NewTorrent {
    id: TorrentId,
    storage_info: StorageInfo,
    piece_hashes: Vec<u8>,
    torrent_tx: torrent::Sender,
  },
  /// Request to eventually write a block to disk.
  WriteBlock {
    id: TorrentId,
    block_info: BlockInfo,
    data: Vec<u8>,
  },
  /// Request to eventually read a block from disk and return it via the
  /// sender.
  ReadBlock {
    id: TorrentId,
    block_info: BlockInfo,
    result_tx: peer::Sender,
  },
  /// Eventually shutdown the disk task.
  Shutdown,
}

/// The entity responsible for saving downloaded file blocks to disk and
/// verifying whether downloaded pieces are valid.
struct Disk {
  /// Each torrent in engine has a corresponding entry in this hashmap, which
  /// includes various metadata about torrent and the torrent specific alert
  /// channel.
  torrents: HashMap<TorrentId, RwLock<Torrent>>,
  /// Port on which disk IO commands are received.
  cmd_rx: Receiver,
  /// Channel on which `Disk` sends alerts to the torrent engine.
  engine_tx: engine::Sender,
}

impl Disk {
  /// Creates a new `Disk` instance and returns a command sender and
  /// an alert receiver.
  fn new(engine_tx: engine::Sender) -> DiskResult<(Self, Sender)> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    Ok((
      Disk {
        torrents: HashMap::new(),
        cmd_rx,
        engine_tx,
      },
      cmd_tx,
    ))
  }

  /// Starts the disk event loop which is run until shutdown or an
  /// unrecoverable error is encountered. (e.g. mpsc channel failure).
  async fn start(&mut self) -> DiskResult<()> {
    log::info!("Starting disk IO event loop");
    while let Some(cmd) = self.cmd_rx.recv().await {
      match cmd {
        Command::NewTorrent {
          id,
          storage_info,
          piece_hashes,
          torrent_tx,
        } => {
          log::trace!(
            "Disk received NetTorrent command: id={}, info={:?}",
            id,
            storage_info
          );
          if self.torrents.contains_key(&id) {
            log::warn!("Torrent {} already allocated", id);

            self.engine_tx.send(engine::Command::TorrentAllocation {
              id,
              result: Err(NewTorrentError::AlreadyExists),
            })?;
            continue;
          }

          // NOTE: Do not return on failure, we don't want to kill
          // the disk task due to potential disk IO errors:
          // we just want to log it and notify engine of it.
          let torrent_res =
            Torrent::new(storage_info, piece_hashes, torrent_tx);
          match torrent_res {
            Ok(torrent) => {
              log::info!("Torrent {} successfully allocated", id);
              self.torrents.insert(id, RwLock::new(torrent));
              self.engine_tx.send(engine::Command::TorrentAllocation {
                id,
                result: Ok(()),
              })?;
            }
            Err(e) => {
              log::error!("Torrent {} allocation failure: {}", id, e,);
              // send notification of allocation failure
              self.engine_tx.send(engine::Command::TorrentAllocation {
                id,
                result: Err(e),
              })?;
            }
          }
        }
        Command::WriteBlock {
          id,
          block_info,
          data,
        } => self.write_block(id, block_info, data).await?,
        Command::ReadBlock {
          id,
          block_info,
          result_tx,
        } => self.read_block(id, block_info, result_tx).await?,
        Command::Shutdown => {
          log::info!("Shutting down disk event loop");
          break;
        }
      }
    }
    Ok(())
  }

  /// Queues a block for writing.
  ///
  /// Returns an error if the torrent id is invalid.
  ///
  /// If the block could not be written dut to IO failure,
  /// the torrent is notified of it.
  async fn write_block(
    &self,
    id: TorrentId,
    block_info: BlockInfo,
    data: Vec<u8>,
  ) -> DiskResult<()> {
    log::trace!("Saving torrent {} block {} to disk", id, block_info);

    // check torrent id
    //
    // TODO: maybe we don't want to crash the disk task due to an invalid
    // torrent id: could it be that disk requests for a torrent arrive after
    // a torrent has been removed?
    let torrent = self.torrents.get(&id).ok_or_else(|| {
      log::error!("Torrent {} not found", id);
      Error::InvalidTorrentId
    })?;
    torrent.write().await.write_block(block_info, data)
  }

  /// Attempts to read a block from disk and return the result via the given
  /// sender.
  ///
  /// Returns an error if the torrent id is invalid.
  ///
  /// If the block could not be read due to IO failure, the torrent is
  /// notified of it.
  async fn read_block(
    &self,
    id: TorrentId,
    block_info: BlockInfo,
    tx: peer::Sender,
  ) -> DiskResult<()> {
    log::trace!("Reading torrent {} block {} from disk", id, block_info);

    // check torrent id
    //
    // TODO: maybe we don't want to crash the disk task due to an invalid
    // torrent id: could it be that disk requests for a torrent arrive after
    // a torrent has been removed?
    let torrent = self.torrents.get(&id).ok_or_else(|| {
      log::error!("Torrent {} not found", id);
      Error::InvalidTorrentId
    })?;
    torrent.read().await.read_block(block_info, tx)
  }
}

#[cfg(test)]
mod tests {
  use std::{fs, path::PathBuf};

  use sha1::{Digest, Sha1};
  use tempfile::tempdir;
  use tokio::sync::mpsc;

  use crate::{blockinfo::block_count, storage_info::FileInfo, BLOCK_LEN};

  use super::*;

  /// Tests the allocation of a torrent, and then the allocation of the same
  /// torrent returning an error.
  #[tokio::test]
  async fn should_allocate_new_torrent() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (_, disk_tx) = spawn(tx).unwrap();

    let Env {
      id,
      piece_hashes,
      info,
      torrent_tx,
      ..
    } = Env::new("allocate_new_torrent");

    // allocate torrent via channel
    disk_tx
      .send(Command::NewTorrent {
        id,
        storage_info: info.clone(),
        piece_hashes: piece_hashes.clone(),
        torrent_tx: torrent_tx.clone(),
      })
      .unwrap();
    // wait for result on alert port
    let alert = rx.recv().await.unwrap();
    assert!(matches!(
      alert,
      engine::Command::TorrentAllocation { result: Ok(()), .. }
    ));

    // check that file was created on disk
    let file = info.files.first().unwrap();
    assert!(info.download_dir.join(&file.path).is_file());

    // try to allocate the same torrent a second time
    disk_tx
      .send(Command::NewTorrent {
        id,
        storage_info: info,
        piece_hashes,
        torrent_tx: torrent_tx.clone(),
      })
      .unwrap();

    // we should get an already exists error
    let alert = rx.recv().await.unwrap();
    assert!(matches!(
      alert,
      engine::Command::TorrentAllocation {
        result: Err(NewTorrentError::AlreadyExists),
        ..
      }
    ));
  }

  /// Tests writing of a complete valid torrent's pieces and verifying that an
  /// alert of each disk write is returned by the disk task.
  #[tokio::test]
  async fn should_write_all_pieces() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (_, disk_tx) = spawn(tx).unwrap();

    let Env {
      id,
      pieces,
      piece_hashes,
      info,
      torrent_tx,
      mut torrent_rx,
    } = Env::new("write_all_pieces");

    // allocate torrent via channel
    disk_tx
      .send(Command::NewTorrent {
        id,
        storage_info: info.clone(),
        piece_hashes: piece_hashes.clone(),
        torrent_tx: torrent_tx.clone(),
      })
      .unwrap();
    // wait for result on alert port
    rx.recv().await.expect("cannot allocate torrent");

    // write all pieces to disk
    for (index, piece) in pieces.iter().enumerate() {
      for_each_block(index, piece.len() as u32, |block| {
        let block_end = block.offset + block.len;
        let data = &piece[block.offset as usize..block_end as usize];
        debug_assert_eq!(data.len(), block.len as usize);
        // //println!(
        //     "Writing piece {index} block {block}"
        // );
        disk_tx
          .send(Command::WriteBlock {
            id,
            block_info: block,
            data: data.to_vec(),
          })
          .unwrap();
      });

      // wait for disk write result
      if let Some(torrent::Command::PieceCompletion(Ok(piece))) =
        torrent_rx.recv().await
      {
        // piece is complete so it should be hashed and valid
        assert_eq!(piece.index, index);
        assert!(piece.is_valid);
      } else {
        panic!("Piece could not be written to disk");
      }
    }

    // clean up test env
    let file = info.files.first().unwrap();
    fs::remove_file(info.download_dir.join(&file.path))
      .expect("cannot clean up disk test torrent file");
  }

  /// Tests writing of an invalid piece and verifying that an alert of it
  /// is returned by the disk task.
  #[tokio::test]
  async fn should_reject_writing_invalid_piece() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (_, disk_tx) = spawn(tx).unwrap();

    let Env {
      id,
      pieces,
      piece_hashes,
      info,
      torrent_tx,
      mut torrent_rx,
    } = Env::new("write_invalid_piece");

    // allocate torrent via channel
    disk_tx
      .send(Command::NewTorrent {
        id,
        storage_info: info.clone(),
        piece_hashes: piece_hashes.clone(),
        torrent_tx: torrent_tx.clone(),
      })
      .unwrap();
    // wait for result on alert port
    rx.recv().await.expect("cannot allocate torrent");

    // write an invalid piece to disk
    let index = 0;
    let invalid_piece: Vec<_> =
      pieces[index].iter().map(|b| b.saturating_add(5)).collect();
    for_each_block(index, invalid_piece.len() as u32, |block| {
      let block_end = block.offset + block.len;
      let data = &invalid_piece[block.offset as usize..block_end as usize];
      debug_assert_eq!(data.len(), block.len as usize);
      //println!("Writing invalid piece {index} block {block}");
      disk_tx
        .send(Command::WriteBlock {
          id,
          block_info: block,
          data: data.to_vec(),
        })
        .unwrap();
    });

    // wait for disk write result
    if let Some(torrent::Command::PieceCompletion(Ok(piece))) =
      torrent_rx.recv().await
    {
      assert_eq!(piece.index, index);
      assert!(!piece.is_valid);
    } else {
      panic!("piece could not be written to disk");
    }
  }

  /// Tests reading of a torrent piece's block and verifying that it is
  /// returned via the provided sender.
  #[tokio::test]
  async fn should_read_piece_blocks() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let (_, disk_tx) = spawn(tx).unwrap();

    let Env {
      id,
      pieces,
      piece_hashes,
      info,
      torrent_tx,
      mut torrent_rx,
    } = Env::new("read_piece_blocks");

    // allocate torrent via channel
    disk_tx
      .send(Command::NewTorrent {
        id,
        storage_info: info.clone(),
        piece_hashes: piece_hashes.clone(),
        torrent_tx: torrent_tx.clone(),
      })
      .unwrap();
    // wait for result on alert port
    rx.recv().await.expect("cannot allocate torrent");

    // write piece to disk
    let index = 1;
    let piece = &pieces[index];
    for_each_block(index, piece.len() as u32, |block| {
      let block_end = block.offset + block.len;
      let data = &piece[block.offset as usize..block_end as usize];
      debug_assert_eq!(data.len(), block.len as usize);
      //println!(
      //     "Writing piece {index} block {block}"
      // );
      disk_tx
        .send(Command::WriteBlock {
          id,
          block_info: block,
          data: data.to_vec(),
        })
        .unwrap();
    });

    // wait for disk write result
    assert!(torrent_rx.recv().await.is_some());

    // set up channels to communicate disk read results
    let (tx, mut rx) = mpsc::unbounded_channel();

    // read each block in piece
    let block_count = block_count(piece.len() as u32) as u32;
    let mut block_offset = 0u32;
    for _ in 0..block_count {
      // when calculating the block length we need to consider that the
      // last block may be smaller than the rest
      let block_len = (piece.len() as u32 - block_offset).min(BLOCK_LEN);
      let block_info = BlockInfo {
        piece_index: index,
        offset: block_offset,
        len: block_len,
      };
      // read block
      disk_tx
        .send(Command::ReadBlock {
          id,
          block_info,
          result_tx: tx.clone(),
        })
        .unwrap();

      // wait for result
      if let Some(peer::Command::Block(block)) = rx.recv().await {
        assert_eq!(block.info(), block_info);
      } else {
        panic!("block could not be read from disk");
      }

      // increment offset for next piece
      block_offset += block_len;
    }

    // clean up test env
    let file = info.files.first().unwrap();
    fs::remove_file(info.download_dir.join(&file.path))
      .expect("cannot clean up disk test torrent file");
  }

  /// Calls the provided function for each block in piece, passing it the
  /// block's `BlockInfo`.
  fn for_each_block(
    piece_index: usize,
    piece_len: u32,
    block_visitor: impl Fn(BlockInfo),
  ) {
    let block_count = block_count(piece_len) as u32;
    // all pieces have four blocks in this test
    debug_assert_eq!(block_count, 4);

    let mut block_offset = 0;
    for _ in 0..block_count {
      // when calculating the block length we need to consider that the
      // last block may be smaller than the rest
      let block_len = (piece_len - block_offset).min(BLOCK_LEN);
      debug_assert!(block_len > 0);
      debug_assert!(block_len <= BLOCK_LEN);

      block_visitor(BlockInfo {
        piece_index,
        offset: block_offset,
        len: block_len,
      });

      // increment offset for next piece
      block_offset += block_len;
    }
  }

  /// The disk IO test environment containing information of a valid torrent.
  struct Env {
    id: TorrentId,
    pieces: Vec<Vec<u8>>,
    piece_hashes: Vec<u8>,
    info: StorageInfo,
    torrent_tx: torrent::Sender,
    torrent_rx: torrent::Receiver,
  }

  impl Env {
    /// Creates a new test environment.
    ///
    /// Tests are run in parallel so multiple environments must not clash,
    /// therefore the test name must be unique, which is included in the
    /// test environment's path. This also helps debugging.
    fn new(test_name: &str) -> Self {
      let id = TorrentId::new();
      let dir = tempdir().unwrap();
      let download_dir = dir.path();
      let download_rel_path =
        PathBuf::from(format!("torrent_disk_test_{test_name}"));
      let piece_len: u32 = 4 * 0x4000;
      // last piece is slightly shorter, to test that it is handled
      // correctly
      let last_piece_len: u32 = piece_len - 935;
      let pieces: Vec<Vec<u8>> = vec![
        (0..piece_len).map(|b| (b % 256) as u8).collect(),
        (0..piece_len)
          .map(|b| b + 1)
          .map(|b| (b % 256) as u8)
          .collect(),
        (0..piece_len)
          .map(|b| b + 2)
          .map(|b| (b % 256) as u8)
          .collect(),
        (0..last_piece_len)
          .map(|b| b + 3)
          .map(|b| (b % 256) as u8)
          .collect(),
      ];
      // build up expected piece hashes
      let mut piece_hashes = Vec::with_capacity(pieces.len() * 20);
      for piece in pieces.iter() {
        let hash = Sha1::digest(piece);
        piece_hashes.extend(hash.as_slice());
      }
      assert_eq!(piece_hashes.len(), pieces.len() * 20);

      // clean up any potential previous test env
      {
        let download_path = download_dir.join(&download_rel_path);
        if download_path.is_file() {
          fs::remove_file(&download_path)
            .expect("cannot clean up previous disk test torrent file");
        } else if download_path.is_dir() {
          fs::remove_dir_all(&download_path)
            .expect("cannot clean up previous disk test torrent dir");
        }
      }

      let download_len = pieces.iter().fold(0, |mut len, piece| {
        len += piece.len() as u64;
        len
      });
      let info = StorageInfo {
        piece_count: pieces.len(),
        piece_len,
        last_piece_len,
        download_len,
        download_dir: download_dir.to_path_buf(),
        files: vec![FileInfo {
          path: download_rel_path,
          torrent_offset: 0,
          len: download_len,
        }],
      };

      let (torrent_tx, torrent_rx) = mpsc::unbounded_channel();

      Self {
        id,
        pieces,
        piece_hashes,
        info,
        torrent_tx,
        torrent_rx,
      }
    }
  }
}
