use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use crate::{error::disk::*, storage_info::FileInfo};

pub struct TorrentFile {
    pub info: FileInfo,
    pub handle: File,
}

impl TorrentFile {
    /// Opens the file in create, read, and write modes at the path of
    /// combining download directory and the path defined in the file info.
    pub fn new(download_dir: &Path, info: FileInfo) -> Result<Self, NewTorrentError> {
        log::trace!(
            "Opening and creating file {:?}, in dir {:?}",
            info,
            download_dir
        );

        let path = download_dir.join(&info.path);
        let handle = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&path)
            .map_err(|e| {
                log::warn!("Failed to open file {:?}", path);
                NewTorrentError::Io(e)
            })?;

        debug_assert!(path.exists());
        Ok(Self { info, handle })
    }
}
