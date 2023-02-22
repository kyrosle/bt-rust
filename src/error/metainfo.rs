pub use serde_bencoded::DeError as BencodeDeError;
pub use serde_bencoded::SerError as BencodeSerError;

pub(crate) type Result<T> = std::result::Result<T, MetainfoError>;

#[derive(thiserror::Error, Debug)]
pub enum MetainfoError {
  #[error("{0}")]
  BencodeDe(BencodeDeError),
  #[error("{0}")]
  BencodeSer(BencodeSerError),

  #[error("Invalid Metainfo")]
  InvalidMetainfo,

  #[error("Invalid Pieces")]
  InvalidPieces,

  #[error("Invalid Tracker Url")]
  InvalidTrackerUrl,
}

impl From<BencodeDeError> for MetainfoError {
  fn from(error: BencodeDeError) -> Self {
    Self::BencodeDe(error)
  }
}

impl From<BencodeSerError> for MetainfoError {
  fn from(error: BencodeSerError) -> Self {
    Self::BencodeSer(error)
  }
}

impl From<url::ParseError> for MetainfoError {
  fn from(_: url::ParseError) -> Self {
    Self::InvalidTrackerUrl
  }
}
