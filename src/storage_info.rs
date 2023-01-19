use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub len: u64,
    pub torrent_offset: u64,
}