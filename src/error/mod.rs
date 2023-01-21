//! Set of module Error
pub mod metainfo;
pub mod peer;
pub mod tracker;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {}
