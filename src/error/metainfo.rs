use serde_bencode::Error  as BencodeError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Bencode(BencodeError),
    #[error("Invalid Metainfo")]
    InvalidMetainfo,
    #[error("Invalid Pieces")]
    InvalidPieces,
    #[error("Invalid Tracker Url")]
    InvalidTrackerUrl,
}

impl From<BencodeError> for Error {
    fn from(error: BencodeError) -> Self {
        Self::Bencode(error)
    }
}

impl From<url::ParseError> for Error {
    fn from(_: url::ParseError) -> Self {
        Self::InvalidTrackerUrl
    }
}
