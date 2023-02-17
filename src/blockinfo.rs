use std::{fmt, ops::Deref, sync::Arc};

use crate::{PieceIndex, BLOCK_LEN};

/// A block is a fixed size chunk of a piece, which in turn is a fixed size
/// chunk of a content. Downloading torrents happen at this block level
/// granularity.
#[derive(
  Clone,
  Copy,
  Debug,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
)]
pub struct BlockInfo {
  /// The index of the piece of which this is a block.
  pub piece_index: PieceIndex,
  /// The zero-based byte offset into the piece.
  pub offset: u32,
  /// The block's length in bytes.
  /// Always 16 Kib (0x4000 bytes) or less, for now.
  pub len: u32,
}

impl BlockInfo {
  /// Returns the index of the block within its pieces, assuming the default
  /// block length of 16 KiB.
  pub fn index_in_piece(&self) -> usize {
    // we need to use "lower than or equal" as this may be the last block
    // in which case it may be shorter than the default block length.
    debug_assert!(self.len <= BLOCK_LEN);
    debug_assert!(self.len > 0);
    (self.offset / BLOCK_LEN) as usize
  }
}

impl fmt::Display for BlockInfo {
  fn fmt(
    &self,
    f: &mut fmt::Formatter<'_>,
  ) -> fmt::Result {
    write!(
      f,
      "(piece: {} offset: {} len: {})",
      self.piece_index, self.offset, self.len
    )
  }
}

/// Returns the length of the block at the index in pieces.
///
/// If the piece is not a multiple of the default block length,
/// the returned value is small.
///
/// # Panics
///
/// Panics if the index multiplied by the default block length would exceed the
/// piece length.
pub fn block_len(
  piece_len: u32,
  block_index: usize,
) -> u32 {
  let block_index = block_index as u32;
  let block_offset = block_index * BLOCK_LEN;
  assert!(piece_len > block_offset);
  std::cmp::min(piece_len - block_offset, BLOCK_LEN)
}

/// Returns the number of blocks in a piece of the given length.
pub fn block_count(piece_len: u32) -> usize {
  // all but the last piece are a multiple of the block length,
  // but the last piece may be shorter so we need to account for this
  // by rounding up before dividing to get the number of blocks in piece.
  (piece_len as usize + (BLOCK_LEN as usize - 1))
    / BLOCK_LEN as usize
}

pub struct Block {
  pub piece_index: PieceIndex,
  pub offset: u32,
  pub data: BlockData,
}

/// Blocks are cached in memory and are shared between the disk task and
/// peer session tasks. Therefore we use atomic references to count to make sure
/// that even if a block is evicted from cache, the peer still using it still has
/// a valid reference to it.
pub type CachedBlock = Arc<Vec<u8>>;

/// Abstracts over the block data type.
///
/// A block may be just a normal byte buffer, or it may be a reference into a cache.
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub enum BlockData {
  Owned(Vec<u8>),
  Cached(CachedBlock),
}

impl BlockData {
  /// Returns the raw block if it's owned.
  ///
  /// # Panics
  ///
  /// This method panics if the block is not owned and is the cache.
  pub fn into_owned(self) -> Vec<u8> {
    match self {
      Self::Owned(b) => b,
      _ => panic!("cannot move block out of cache"),
    }
  }
}

impl Deref for BlockData {
  type Target = [u8];
  fn deref(&self) -> &Self::Target {
    match self {
      BlockData::Owned(b) => b.as_ref(),
      BlockData::Cached(b) => b.as_ref(),
    }
  }
}

impl From<Vec<u8>> for BlockData {
  fn from(value: Vec<u8>) -> Self {
    Self::Owned(value)
  }
}

impl From<CachedBlock> for BlockData {
  fn from(value: CachedBlock) -> Self {
    Self::Cached(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // An arbitrary piece length that is an exact multiple of the canonical
  // block length (16 KiB).
  const BLOCK_LEN_MULTIPLE_PIECE_LEN: u32 =
    2 * BLOCK_LEN;

  // An arbitrary piece length that is _not_ a multiple of the canonical block
  // length and the amount with which it overlaps the nearest exact multiple
  // value.
  const OVERLAP: u32 = 234;
  const UNEVEN_PIECE_LEN: u32 =
    2 * BLOCK_LEN + OVERLAP;

  #[test]
  fn test_block_len() {
    assert_eq!(
      block_len(BLOCK_LEN_MULTIPLE_PIECE_LEN, 0),
      BLOCK_LEN
    );
    assert_eq!(
      block_len(BLOCK_LEN_MULTIPLE_PIECE_LEN, 1),
      BLOCK_LEN
    );

    assert_eq!(
      block_len(UNEVEN_PIECE_LEN, 0),
      BLOCK_LEN
    );
    assert_eq!(
      block_len(UNEVEN_PIECE_LEN, 1),
      BLOCK_LEN
    );
    assert_eq!(
      block_len(UNEVEN_PIECE_LEN, 2),
      OVERLAP
    );
  }

  #[test]
  #[should_panic]
  fn test_block_len_invalid_index_panic() {
    block_len(BLOCK_LEN_MULTIPLE_PIECE_LEN, 2);
  }

  #[test]
  fn test_block_count() {
    assert_eq!(
      block_count(BLOCK_LEN_MULTIPLE_PIECE_LEN),
      2
    );

    assert_eq!(block_count(UNEVEN_PIECE_LEN), 3);
  }
}
