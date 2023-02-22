use crate::error::metainfo::{BencodeDeError, BencodeSerError};
use reqwest::Error as HttpError;

pub type Result<T, E = TrackerError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
  #[error("{0}")]
  BencodeDe(BencodeDeError),
  #[error("{0}")]
  BencodeSer(BencodeSerError),

  #[error("{0}")]
  Http(HttpError),
}

impl From<BencodeDeError> for TrackerError {
  fn from(error: BencodeDeError) -> Self {
    Self::BencodeDe(error)
  }
}

impl From<BencodeSerError> for TrackerError {
  fn from(error: BencodeSerError) -> Self {
      Self::BencodeSer(error) 
  }
}

impl From<HttpError> for TrackerError {
  fn from(value: HttpError) -> Self {
    Self::Http(value)
  }
}
