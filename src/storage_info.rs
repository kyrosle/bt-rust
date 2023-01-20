use std::path::PathBuf;

/// Information about the torrent file.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// file's relative path from the download directory.
    pub path: PathBuf,
    /// the file's length
    pub len: u64,
    /// The byte offset of the file within the torrent, when all files in
    /// torrent are viewed as a single contiguous byte array. This is always
    /// 0 for a single file torrent.
    pub torrent_offset: u64,
}
