use crate::error::metainfo::BencodeError;
use reqwest::Error as HttpError;
#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("{0}")]
    Bencode(BencodeError),
    #[error("{0}")]
    Http(HttpError)
}

impl From<BencodeError> for TrackerError {
    fn from(value: BencodeError) -> Self {
        Self::Bencode(value)
    }
}

impl From<HttpError> for TrackerError {
    fn from(value: HttpError) -> Self {
        Self::Http(value)
    }
}