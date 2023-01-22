//! Set of module Error
pub mod metainfo;
pub mod peer;
pub mod tracker;
pub mod torrent;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {}
