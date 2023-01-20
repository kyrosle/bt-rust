use std::fmt;
use std::path::{Path, PathBuf};

use url::Url;

use crate::error::metainfo::MetainfoError;
use crate::storage_info::FileInfo;
use crate::Sha1Hash;

pub(crate) type Result<T> = std::result::Result<T, MetainfoError>;

/// The meta info from torrent file.
#[derive(Clone)]
pub struct Metainfo {
    /// torrent name, the form for download path.
    pub name: String,
    /// 20 bytes of SHA-1, 
    /// Used for verifying the info. 
    /// structure: [20]bytes
    pub info_hash: Sha1Hash,
    /// contain the a concatenation of each piece's SHA-1,
    /// length is the multiple of 20 bytes.
    /// formed ordered by the file in files dictionary.
    pub pieces: Vec<u8>,
    /// the length of the pieces
    pub piece_len: usize,
    /// A list of strings corresponding to subdirectory names,
    /// the last of which is the actual file name
    pub files: Vec<FileInfo>,
    /// The trackers that we can announce to.
    pub trackers: Vec<Url>,
}

impl fmt::Debug for Metainfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metainfo")
            .field("name", &self.name)
            .field("info_hash", &self.info_hash)
            .field("pieces", &"<pieces...>")
            .field("piece_len", &self.piece_len)
            .field("structure", &self.files)
            .finish()
    }
}

impl Metainfo {
    /// Parse from a byte buffer to crate a [`Metainfo`] instance
    /// or return a Error about the invalid format, syntax or which come from `serde_bencode`
    /// 
    /// Here are some rules: 
    /// - the bencode format and syntax should correct.
    /// - the length of pieces in info should be the multiple of 20.
    /// - cannot not contain both `length` (single file) and `files` (multi files).
    /// - the 'len' filed in info should not less than 0.
    /// - If having multi files, the `files` should not be empty.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        // parse the file and then do verification.
        let metainfo: raw::Metainfo = serde_bencode::from_bytes(bytes)?;

        // the pieces field is a concatenation of 20 byte SHA-1 hashes, so it
        // must be a multiple of 20
        if metainfo.info.pieces.len() % 20 != 0 {
            return Err(MetainfoError::InvalidMetainfo);
        }

        // verify download structure and build up files metadata
        let mut files = Vec::new();
        if let Some(len) = metainfo.info.len {
            if metainfo.info.files.is_some() {
                log::warn!("Metainfo cannot contain both `length` and `files`");
                return Err(MetainfoError::InvalidMetainfo);
            }
            if len == 0 {
                log::warn!("File length is 0");
                return Err(MetainfoError::InvalidMetainfo);
            }

            // the path of this file is just the torrent name
            files.push(FileInfo {
                path: metainfo.info.name.clone().into(),
                len,
                torrent_offset: 0,
            });
        } else if let Some(raw_files) = &metainfo.info.files {
            if raw_files.is_empty() {
                log::warn!("Metainfo files must not be empty");
                return Err(MetainfoError::InvalidMetainfo);
            }

            files.reserve_exact(raw_files.len());

            // the offset of series of files
            let mut torrent_offset = 0;
            for file in raw_files.iter() {
                // verify the file length is non-zero
                if file.len == 0 {
                    log::warn!("File {:?} length is 0", file.path);
                    return Err(MetainfoError::InvalidMetainfo.into());
                }

                // verify that the path is not empty
                let path: PathBuf = file.path.iter().collect();
                if path.as_os_str().is_empty() {
                    log::warn!("Path in metainfo is empty");
                    return Err(MetainfoError::InvalidMetainfo.into());
                }

                // verify that the path is not absolute
                if path.is_absolute() {
                    log::warn!("Path {:?} is absolute", path);
                    return Err(MetainfoError::InvalidMetainfo.into());
                }

                // verify that the path is not the root
                if path == Path::new("/") {
                    log::warn!("Path {:?} is root", path);
                    return Err(MetainfoError::InvalidMetainfo.into());
                }

                // file is now verified, we can collect it
                files.push(FileInfo {
                    path,
                    torrent_offset,
                    len: file.len,
                });

                // advance offset for next file
                torrent_offset += file.len;
            }
        } else {
            log::warn!("No `length` or `files` key present in metainfo");
            return Err(MetainfoError::InvalidMetainfo.into());
        }

        let mut trackers = Vec::new();
        if !metainfo.announce_list.is_empty() {
            let tracker_count = metainfo
                .announce_list
                .iter()
                .map(|t| t.len())
                .sum::<usize>()
                + metainfo.announce.as_ref().map(|_| 1).unwrap_or_default();
            trackers.reserve(tracker_count);

            for announce in metainfo.announce_list.iter() {
                for tracker in announce.iter() {
                    let url = Url::parse(tracker)?;

                    // may use UDP ???
                    if url.scheme() == "http" || url.scheme() == "https" {
                        trackers.push(url);
                    }
                }
            }
        } else if let Some(tracker) = &metainfo.announce {
            let url = Url::parse(tracker)?;
            if url.scheme() == "http" || url.scheme() == "https" {
                trackers.push(url);
            }
        }

        if trackers.is_empty() {
            log::warn!("No HTTP trackers in metainfo");
        }

        // create the info hash.
        let info_hash = metainfo.crate_info_hash()?;

        Ok(Metainfo {
            name: metainfo.info.name,
            info_hash,
            pieces: metainfo.info.pieces,
            piece_len: metainfo.info.piece_len,
            files,
            trackers,
        })
    }

    /// Return true if the download multi files
    pub fn is_archive(&self) -> bool {
        self.files.len() > 1
    }
}

mod raw {
    //! Only for `bencode` crate deserialize to
    //! convert into ``
    use serde_derive::{Deserialize, Serialize};
    use sha1::Digest;

    use super::*;
    use crate::Sha1Hash;

    /// Details field meaning in [.torrent file](https://en.wikipedia.org/wiki/Torrent_file)
    #[derive(Debug, Deserialize)]
    pub struct Metainfo {
        /// this maps to a dictionary whose keys are dependent on whether one or more files are being shared
        pub info: Info,
        /// the URL of the tracker
        pub announce: Option<String>,
        #[serde(default)]
        #[serde(rename = "announce-list")]
        pub announce_list: Vec<Vec<String>>,
    }

    impl Metainfo {
        pub fn crate_info_hash(&self) -> Result<Sha1Hash> {
            let info = serde_bencode::to_bytes(&self.info)?;
            let digest = sha1::Sha1::digest(&info);
            let mut info_hash = [0; 20];
            info_hash.copy_from_slice(&digest);
            Ok(info_hash)
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Info {
        /// suggested filename where the file is to be saved (if one file)/suggested directory name
        /// where the files are to be saved (if multiple files)
        pub name: String,
        #[serde(with = "serde_bytes")]
        /// a hash list, i.e., a concatenation of each piece's SHA-1 hash. As SHA-1 returns a 160-bit hash, 
        /// pieces will be a string whose length is a multiple of 20 bytes.
        /// If the torrent contains multiple files, 
        /// the pieces are formed by concatenating the files in the order they appear in the files dictionary 
        /// (i.e., all pieces in the torrent are the full piece length except for the last piece, which may be shorter).
        pub pieces: Vec<u8>,
        #[serde(rename = "piece length")]
        /// number of bytes per piece. This is commonly 28 KiB = 256 KiB = 262,144 B
        pub piece_len: usize,
        #[serde(rename = "length")]
        /// size of the file in bytes (only when one file is being shared though)
        pub len: Option<u64>,
        /// a list of dictionaries each corresponding to a file (only when multiple files are being shared)
        pub files: Option<Vec<File>>,
        /// not used filed but kept in here,
        /// maybe for encode back a valid info hash for hashing.
        pub private: Option<u8>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct File {
        /// a list of strings corresponding to subdirectory names, the last of which is the actual file name
        pub path: Vec<String>,
        #[serde(rename = "length")]
        /// size of the file in bytes
        pub len: u64,
    }
}
