pub use serde_bencode::Error as BencodeError;

pub(crate) type Result<T> = std::result::Result<T, MetainfoError>;

#[derive(thiserror::Error, Debug)]
pub enum MetainfoError {
  #[error("{0}")]
  Bencode(BencodeError),

  #[error("Invalid Metainfo")]
  InvalidMetainfo,

  #[error("Invalid Pieces")]
  InvalidPieces,

  #[error("Invalid Tracker Url")]
  InvalidTrackerUrl,
}

impl From<BencodeError> for MetainfoError {
  fn from(error: BencodeError) -> Self {
    Self::Bencode(error)
  }
}

impl From<url::ParseError> for MetainfoError {
  fn from(_: url::ParseError) -> Self {
    Self::InvalidTrackerUrl
  }
}
