//! There 4 situations of edge case as below:
//!
//! ## Unbounded:
//!
//! ```text
//! -------------------
//! | file slice: 16  |
//! ------------------
//! | block: 16      ^
//! ----------------^
//!                ^
//!            split here
//! ```
//!
//! ## Split at Buffer boundary:
//!
//! ```text
//! -------------------
//! | file slice: 16  |
//! -----------------------------------
//! | block: 16      ^ block: 16     |
//! ----------------^-----------------
//!                ^            
//!            split here
//! ```
//!
//! ## Split within buffer:
//!
//! ```text
//! ------------------------------
//! | file slice: 25             |
//! -----------------------------------
//! | block: 16      | block: 16 ^    |
//! -----------------------------^-----
//!                              ^
//!                          split here
//! ```
//!
//! ## Unbounded:
//!
//! ```text
//! ------------------------------------------
//! | file slice: 40                         |
//! -----------------------------------------
//! | block: 16      | block: 16     | ^
//! ---------------------------------- ^
//!                                    ^
//!                                 split here
//! ```

use super::IoVec;

/// Wrapper over a slice of [`IoVec`]s that provides zero-copy functionality to
/// pass only a sub-slice of the iovecs to vectored IO functions.
#[derive(Debug)]
pub struct IoVecs<'a> {
  /// The entire view of the underlying buffers.
  bufs: &'a mut [IoVec],
  /// If set, the buffer is bounded by a given boundary, and is effectively
  /// "split". This includes metadata to reconstruct the second half of the
  /// split.
  split: Option<Split>,
}

impl<'a> IoVecs<'a> {
  /// Bounds the iovecs, potentially splitting it in two, if the total byte
  /// count of the buffers exceeds the limit.
  ///
  /// # Arguments
  ///
  /// * `bufs` - A slice that points to a contiguous list of IO vectors, which
  ///     in turn point to the actual blocks of memory used for file IO.
  /// * `max_len` - The maximum byte count of the total number of bytes in the
  ///     IO vectors.
  ///
  /// # Panics
  ///
  /// The constructor panics if the max length is 0.
  pub fn bounded(bufs: &'a mut [IoVec], max_len: usize) -> Self {
    assert!(max_len > 0, "IoVecs max length should be larger than 0.");

    // Detected whether the total byte count in bufs exceeds the slice
    // length by accumulating the buffer lengths and stopping at the buffer whose
    // accumulated length exceeds the slice length.
    let mut bufs_len = 0;
    let bufs_split_pos = match bufs.iter().position(|buf| {
      bufs_len += buf.as_slice().len();
      bufs_len >= max_len
    }) {
      Some(pos) => pos,
      None => return Self::unbounded(bufs),
    };

    // //println!("{max_len},{bufs_len},{bufs_split_pos}");

    // If we're here, it means that the total buffers length exceeds the
    // slice length and we must split the buffers.
    if bufs_len == max_len {
      // The buffer boundary aligns with the file boundary. There are two
      // case here:
      // 1. the buffer are the same length as the file, in which case
      //      there is nothing to split.
      // 2. or we just need to split at the buffer boundary.
      if bufs_split_pos + 1 == bufs.len() {
        // the split position is the end of the last buffer, so
        // there is nothing to split
        Self::unbounded(bufs)
      } else {
        // we can split at the buffer boundary
        Self::split_at_buffer_boundary(bufs, bufs_split_pos)
      }
    } else {
      // Otherwise the buffer boundary does not align with the file
      // boundary (the situations of `Split within buffer`),
      // so we must trim the iovec that is at the file boundary.

      // Find the position where we need to split the iovec.
      // We need the relative offset in the buffer within all buffers and
      // then subtracting that from the file length.
      // (TODO: encapsulation the splitting position logic)
      let buf_to_split = bufs[bufs_split_pos].as_slice();
      let buf_offset = bufs_len - buf_to_split.len();
      let buf_split_pos = max_len - buf_offset;
      debug_assert!(buf_split_pos < buf_to_split.len());

      Self::split_within_buffer(bufs, bufs_split_pos, buf_split_pos)
    }
  }

  /// Creates an unbounded `IoVec`, meaning that no split is necessary.
  pub fn unbounded(bufs: &'a mut [IoVec]) -> Self {
    IoVecs { bufs, split: None }
  }

  /// Creates a "clean split", in which the split occurs at the buffer
  /// boundary and `bufs` need only be split at the slice level.
  fn split_at_buffer_boundary(bufs: &'a mut [IoVec], pos: usize) -> Self {
    IoVecs {
      bufs,
      split: Some(Split {
        pos,
        split_buf_second_half: None,
      }),
    }
  }

  /// Creates a split where the split occurs within one of the buffers of `bufs`
  ///
  /// # Arguments
  ///
  /// * `bufs`: the whole buffers.
  ///
  /// * `split_pos`: the index of splitting buffer.
  ///
  /// * `buf_split_pos`: the position that should split at the splitting buffer position.
  fn split_within_buffer(
    bufs: &'a mut [IoVec],
    split_pos: usize,
    buf_split_pos: usize,
  ) -> Self {
    // save the original slice at the boundary, so that later we can
    // restore it.
    let buf_to_split = bufs[split_pos].as_slice();

    // trim the overhanging part off the iovec.
    let (split_buf_first_half, split_buf_second_half) =
      buf_to_split.split_at(buf_split_pos);
    // //println!(
    //     "split first : {split_buf_first_half:?}"
    // );
    // //println!(
    //     "split second: {split_buf_second_half:?}"
    // );
    // //println!(
    //     "ptr: {:p}",
    //     split_buf_second_half.as_ptr()
    // );

    // We need to convert the second half of the split buffer into its
    // raw representation, as we can't store a reference to it as well as
    // store mutable references to the rest of the buffer in `IoVecs`.
    //
    // This is safe:
    // 1. The second half of the buffer is not used until the buffer is
    //      reconstructed.
    // 2. And we don't leak the raw buffer or pointers for other code to
    //      unsafely reconstruct the slice. The slice is only reconstructed
    //      in `IoVecs::into_second_half`, assigning it to the `IoVec` at
    //      `split_post`(splitting buffer index) in `bufs`,
    //      without touching its underlying memory.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let split_buf_second_half = RawBuf {
      ptr: split_buf_second_half.as_ptr(),
      len: split_buf_second_half.len(),
    };
    #[cfg(target_os = "windows")]
    let split_buf_second_half = RawBuf {
      ptr: split_buf_second_half.to_vec(),
    };

    // Shrink the iovec at the file boundary:
    //
    // Here we need to use unsafe code as there is no way to borrow
    // a slice from `bufs` (`buf_to_split` above), and then assigning
    // that same slice to another element of bufs below, as that would
    // be an immutable and mutable borrow at the same time, breaking
    // aliasing rules.
    //
    // However, it is safe to do so, as we're not actually touching the
    // underlying byte buffer that the slice refers to, but simply replacing
    // the `IoVec` at `split_pos` in `buf`, i.e. shrinking the slice
    // itself, not the memory region pointed to by the slice.
    let split_buf_first_half = unsafe {
      std::slice::from_raw_parts(
        split_buf_first_half.as_ptr(),
        split_buf_first_half.len(),
      )
    };
    bufs[split_pos] = IoVec::from_slice(split_buf_first_half);

    IoVecs {
      bufs,
      split: Some(Split {
        pos: split_pos,
        split_buf_second_half: Some(split_buf_second_half),
      }),
    }
  }

  /// Returns an immutable slice to the iovecs in the `first half` of the split.
  #[inline]
  pub fn as_slice(&self) -> &[IoVec] {
    if let Some(split) = &self.split {
      // due to `Self::advance` it may be that the first half off the
      // split is actually empty, in which case we need to return an
      // empty slice
      if split.pos == 0
        && !self.bufs.is_empty()
        && self.bufs[0].as_slice().is_empty()
      {
        &self.bufs[0..0]
      } else {
        // we need to include the buffer under the split position too
        &self.bufs[0..=split.pos]
      }
    } else {
      self.bufs
    }
  }

  /// Return a u8 vector.
  pub fn as_u8_vec(&self) -> Vec<u8> {
    let slice = self.as_slice();
    slice
      .iter()
      .map(|s| s.as_slice())
      .flat_map(|s| s.to_vec())
      .collect::<Vec<_>>()
  }

  /// Advances the internal cursor of the iovecs slice.
  ///
  /// # Notes
  ///
  /// Elements in the slice may be modified if the cursor is not advanced to
  /// the end of the slice. For example if we have a slice of buffers with 2
  /// `IoVec`s, both of length 8, and we advance the cursor by 10 bytes the
  /// first `IoVec` will be untouched however the second will be modified to
  /// remove the first 2 bytes.
  ///
  /// # Panics
  ///
  /// Panics if `n` is larger than the combined byte count of the
  /// buffers (first half of the split if there is any, or bytes in all
  /// buffers if there is not).
  #[inline]
  pub fn advance(&mut self, n: usize) {
    // This is mostly borrowed from:
    //
    // However, there is quite a bit more complexity here due to the iovecs
    // potentially being split. For one, we must not advance past the split
    // boundary, and because the iovecs may be split either on a buffer
    // boundary or within a buffer, the edge cases crop up here.
    //
    // What we do is we first count how many whole buffers can be trimmed
    // off from `self.as_slice()` (which returns only the first half of the
    // split, if there is any), and then determine whether it is allowed to
    // advance the given amount of bytes.
    //
    // Assuming `n` is for the whole first part of the split (the edge cases
    // arise when near the split boundary), even if we're on a buffer
    // boundary, we can not just trim off all buffers up until the boundary,
    // because then, when returning the first half in `self.as_slice()` we
    // would not be able to determine wether to return an empty slice or
    // not. Either way, we need to keep the buffer at the split position, as
    // some bytes of that buffer are either in the second half of the split,
    // or the buffer is empty which we can use as a marker for returning and
    // empty slice.

    // the number of buffers to remove.
    let mut bufs_to_remove_count = 0;
    // the total length of bytes from buffers which removed.
    let mut total_remove_len = 0;

    // count the whole buffers to remove.
    for buf in self.as_slice().iter() {
      let buf_len = buf.as_slice().len();
      // if the last byte to be removed is in this buffer, don't remove
      // buffer, we just need to adjust its offset.
      if total_remove_len + buf_len > n {
        break;
      } else {
        // otherwise there are more bytes to remove than this buffer,
        // ergo we want to remove it.
        total_remove_len += buf_len;
        bufs_to_remove_count += 1;
      }
    }

    // if there is a split and we want to trim off the whole first half,
    // we must keep the buffer at the split position.
    if let Some(split) = &self.split {
      if bufs_to_remove_count == split.pos + 1 {
        if n > total_remove_len {
          panic!("cannot advance iovecs by more than buffers length");
        }

        bufs_to_remove_count -= 1;
        total_remove_len -= self
          .as_slice()
          .last()
          .map(|s| s.as_slice().len())
          .unwrap_or(0);
      }
    }

    // trim buffers off the front of `self.bufs`
    //
    // hack: We need the original lifetime of the slice and not the
    // re-borrowed temporary lifetime of `&mut self` passed to this function
    // (as would happen by re-assigning a sub-slice of `self.bufs` to itself),
    // so we move the `bufs` slice out of `self` by value with the original
    // lifetime and take the slice from that.
    let bufs = std::mem::take(&mut self.bufs);
    self.bufs = &mut bufs[bufs_to_remove_count..];

    // if there is a split, also adjust the split position.
    if let Some(split) = &mut self.split {
      if bufs_to_remove_count >= split.pos {
        split.pos = 0;
      } else {
        split.pos -= bufs_to_remove_count;
      }
    }

    // if there are buffers left, it may be that the first buffer needs some
    // bytes trimmed off its front.
    if !self.bufs.is_empty() {
      // adjust the advance count.
      let n = n - total_remove_len;
      if n > 0 {
        let slice = self.bufs[0].as_slice();
        assert!(slice.len() >= n);
        let ptr = slice.as_ptr();
        let slice =
          unsafe { std::slice::from_raw_parts(ptr.add(n), slice.len() - n) };
        self.bufs[0] = IoVec::from_slice(slice);
      }
    }
  }

  /// Returns the second half of the split, reconstructing the split buffer in
  /// the middle, if necessary, consuming the split in the process.
  #[inline]
  pub fn into_tail(self) -> &'a mut [IoVec] {
    if let Some(mut second_half) = self.split {
      // If the buffer at the boundary was split, we need to restore it
      // first. Otherwise, the buffers were split at a buffer boundary
      // so we can just return the second half of the split.
      if let Some(split_buf_second_half) = second_half.split_buf_second_half {
        // See note in `Self::split_within_buffer`:
        // the pointers here refer to the same buffer at `bufs[split_pos]`,
        // so all we're doing is resizing the slice at that position to be the
        // second half of the original slice that was untouched since creating
        // this split.
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let split_buf_second_half = unsafe {
          let slice = std::slice::from_raw_parts(
            split_buf_second_half.ptr,
            split_buf_second_half.len,
          );
          IoVec::new(slice)
        };
        #[cfg(target_os = "windows")]
        let split_buf_second_half = IoVec::from_vec(split_buf_second_half.ptr);

        // //println!(
        //     "crated from split: {split_buf_second_half:?}"
        // );

        // restore the second half of the split buffer
        self.bufs[second_half.pos] = split_buf_second_half;
      } else {
        second_half.pos += 1;
      }

      // return a slice to the buffers starting at the split position
      &mut self.bufs[second_half.pos..]
    } else {
      // otherwise there is no second half, so we return an empty slice
      let write_buf_len = self.bufs.len();
      &mut self.bufs[write_buf_len..]
    }
  }
}

/// Represents the second half of a `&mut [IoVec<&[u8]>]` split int two,
/// where the split may not be on the boundary of two buffers.
///
/// The complication arises from the fact that the split may not be on a buffer
/// boundary, but we want to perform the split by keeping the original slices
/// (i.e. without allocating a new vector). This requires keeping the
/// first part of the slice, the second part of the slice, and if the split
/// occurred within a buffer, a copy of the second half of that split buffer.
///
/// This way, the user can use the first half of the buffers to pass it for
/// vectored IO, (using the [`std::io::Write::write_vectored`], don't know that the
/// performance would be like the `writev` in linux platforms??).
///
#[derive(Debug)]
struct Split {
  /// The position of the buffer in which the split occurred, either
  /// within the buffer or one past the end of the buffer. This means that
  /// this position includes the last buffer of the first half of the split, that
  /// is, we would split at `[0, pos]`.
  pos: usize,
  /// If set, it means that the buffer at `bufs[split_pos]` was further split
  /// in two. It contains the second half of the split buffer.
  split_buf_second_half: Option<RawBuf>,
}

/// A byte slice deconstructed into its raw parts.
#[derive(Debug)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
struct RawBuf {
  ptr: *const u8,
  len: usize,
}

#[derive(Debug)]
#[cfg(target_os = "windows")]
struct RawBuf {
  ptr: Vec<u8>,
}

/// This function is analogous to [`std::io::IoVec::advance`](windows), expect
/// that it works on a list of mutable iovec buffers,
/// while the former is for an immutable list of such buffers.
///
/// The reason this is separate is because there is no need for the `IoVecs`
/// abstraction when working with vectored read IO: `preadv`
/// (in linux system it may be ReadFileScatter in windows system)
/// only read as much from files as the buffer have capacity for.
/// This is in fact symmetrical to how `pwritev` works, which writes as much as
/// is available in the buffers.
/// However, it has the effect that it may extend the file size, which is what
/// `IoVec` guards against. Since this protection is not necessary for reads,
/// but advancing the buffer cursor is, a free function is available for this purpose.
pub fn advance(bufs: &mut [IoVec], n: usize) -> &mut [IoVec] {
  // number of buffers to remove.
  let mut bufs_to_remove_count = 0;
  // total length of all the to be removed buffers.
  let mut total_removed_len = 0;

  for buf in bufs.iter() {
    let buf_len = buf.as_slice().len();
    // if the last byte to the removed is in this buffer,
    // don't remove buffer, we just need to adjust its offset
    if total_removed_len + buf_len > n {
      break;
    } else {
      // otherwise there are more bytes to remove than this buffer,
      // ergo we want to remove it.
      total_removed_len += buf_len;
      bufs_to_remove_count += 1;
    }
  }
  let bufs = &mut bufs[bufs_to_remove_count..];

  // if not all buffers were removed, check if we need to trim
  // more bytes from this buffer.
  if !bufs.is_empty() {
    let buf = bufs[0].as_slice();
    let offset = n - total_removed_len;

    let slice = unsafe {
      std::slice::from_raw_parts_mut(
        buf.as_ptr().add(offset) as *mut u8,
        buf.len() - offset,
      )
    };
    let _ = std::mem::replace(&mut bufs[0], IoVec::from_slice(slice));
  }
  bufs
}

#[cfg(test)]
mod tests {
  use super::*;

  /// Tests that splitting of the blocks that align with the file boundary at
  /// the last block is a noop.
  ///
  /// -----------------------------------
  /// | file slice: 32                  |
  /// -----------------------------------
  /// | block: 16      | block: 16      |
  /// -----------------------------------
  #[test]
  fn should_not_split_buffers_same_size_as_file() {
    let file_len = 32;
    let blocks =
      vec![(0..16).collect::<Vec<u8>>(), (16..32).collect::<Vec<u8>>()];
    let blocks_len: usize = blocks.iter().map(Vec::len).sum();

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let iovecs = IoVecs::bounded(&mut bufs, file_len);

    // we should have both buffers
    assert_eq!(iovecs.as_slice().len(), 2);
    // there was no split
    assert!(iovecs.split.is_none());

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half has the same bytes as the blocks
    let expected_first_half: Vec<_> = blocks.iter().flatten().collect();
    assert_eq!(first_half.len(), file_len);
    assert_eq!(first_half.len(), blocks_len);
    assert_eq!(first_half, expected_first_half);

    // restore the second half of the split buffer, which should be empty
    let second_half = iovecs.into_tail();
    assert!(second_half.is_empty());
  }

  /// Tests that splitting of the blocks whose combined length is smaller than
  /// that of the file is a noop.
  ///
  /// --------------------------------------------
  /// | file slice: 42                           |
  /// --------------------------------------------
  /// | block: 16      | block: 16      |
  /// -----------------------------------
  #[test]
  fn should_not_split_buffers_smaller_than_file() {
    let file_len = 42;
    let blocks =
      vec![(0..16).collect::<Vec<u8>>(), (16..32).collect::<Vec<u8>>()];
    let blocks_len: usize = blocks.iter().map(Vec::len).sum();

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let iovecs = IoVecs::bounded(&mut bufs, file_len);

    // we should have both buffers
    assert_eq!(iovecs.as_slice().len(), 2);
    // there was no split
    assert!(iovecs.split.is_none());

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half has the same bytes as the blocks
    let expected_first_half: Vec<_> = blocks.iter().flatten().collect();
    assert_eq!(first_half.len(), blocks_len);
    assert_eq!(first_half, expected_first_half);

    // restore the second half of the split buffer, which should be empty
    let second_half = iovecs.into_tail();
    assert!(second_half.is_empty());
  }

  /// Tests splitting of the blocks that do not align with file boundary at the
  /// last block.
  ///
  /// ------------------------------
  /// | file slice: 25             |
  /// -----------------------------------
  /// | block: 16      | block: 16 ^    |
  /// -----------------------------^-----
  ///                              ^
  ///              split here into 9 and 7 long halves
  #[test]
  fn should_split_last_buffer_not_at_boundary() {
    let file_len = 25;
    let blocks =
      vec![(0..16).collect::<Vec<u8>>(), (16..32).collect::<Vec<u8>>()];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let iovecs = IoVecs::bounded(&mut bufs, file_len);

    // we should have both buffers
    assert_eq!(iovecs.as_slice().len(), 2);

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half is just the file slice number of bytes
    let expected_first_half: Vec<_> =
      blocks.iter().flatten().take(file_len).collect();
    assert_eq!(first_half.len(), file_len);
    assert_eq!(first_half, expected_first_half);

    // restore the second half of the split buffer
    let second_half = iovecs.into_tail();
    // compare the contents of the second half of the split: convert it
    // to a flat vector for easier comparison
    let second_half: Vec<_> =
      second_half.iter().flat_map(|i| i.as_slice()).collect();
    assert_eq!(second_half.len(), 7);
    // the expected second half is just the bytes after the file slice number of bytes
    let expected_second_half: Vec<_> =
      blocks.iter().flatten().skip(file_len).collect();
    assert_eq!(second_half, expected_second_half);
  }

  /// Tests splitting of the blocks that do not align with file boundary.
  ///
  /// ------------------------------
  /// | file slice: 25             |
  /// ----------------------------------------------------
  /// | block: 16      | block: 16 ^    | block: 16      |
  /// -----------------------------^----------------------
  ///                              ^
  ///              split here into 9 and 7 long halves
  #[test]
  fn should_split_middle_buffer_not_at_boundary() {
    let file_len = 25;
    let blocks = vec![
      (0..16).collect::<Vec<u8>>(),
      (16..32).collect::<Vec<u8>>(),
      (32..48).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let iovecs = IoVecs::bounded(&mut bufs, file_len);

    // we should have only the first two buffers
    assert_eq!(iovecs.as_slice().len(), 2);
    assert!(iovecs.split.is_some());

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half is just the file slice number of bytes
    let expected_first_half: Vec<_> =
      blocks.iter().flatten().take(file_len).collect();
    assert_eq!(first_half.len(), file_len);
    assert_eq!(first_half, expected_first_half);

    // restore the second half of the split buffer
    let second_half = iovecs.into_tail();
    // compare the contents of the second half of the split: convert it to
    // a flat vector for easier comparison
    let second_half: Vec<_> =
      second_half.iter().flat_map(|i| i.as_slice()).collect();
    // the length should be the length of the second half the split buffer
    // as well as the remaining block's length
    assert_eq!(second_half.len(), 7 + 16);
    // the expected second half is just the bytes after the file slice number of bytes
    let expected_second_half: Vec<_> =
      blocks.iter().flatten().skip(file_len).collect();
    assert_eq!(second_half, expected_second_half);
  }

  /// Tests that advancing only a fraction of the first half of the split does
  /// not affect the rest of the buffers.
  #[test]
  fn partial_advance_in_first_half_should_not_affect_rest() {
    let file_len = 25;
    let blocks = vec![
      (0..16).collect::<Vec<u8>>(),
      (16..32).collect::<Vec<u8>>(),
      (32..48).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let mut iovecs = IoVecs::bounded(&mut bufs, file_len);

    // advance past the first buffer (less then the whole write buffer/file
    // length)
    let advance_count = 18;
    iovecs.advance(advance_count);

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half is just the file slice number of bytes
    let expected_first_half: Vec<_> = blocks
      .iter()
      .flatten()
      .take(file_len)
      .skip(advance_count)
      .collect();
    assert_eq!(first_half, expected_first_half);

    // restore the second half of the split buffer, which shouldn't be
    // affected by the above advance
    let second_half = iovecs.into_tail();
    // compare the contents of the second half of the split: convert it to
    // a flat vector for easier comparison
    let second_half: Vec<_> =
      second_half.iter().flat_map(|i| i.as_slice()).collect();
    // the length should be the length of the second half the split buffer
    // as well as the remaining block's length
    assert_eq!(second_half.len(), 7 + 16);
    // the expected second half is just the bytes after the file slice number of bytes
    let expected_second_half: Vec<_> =
      blocks.iter().flatten().skip(file_len).collect();
    assert_eq!(second_half, expected_second_half);
  }

  /// Tests that advancing only a fraction of the first half of the split in
  /// multiple steps does not affect the rest of the buffers.
  #[test]
  fn advances_in_first_half_should_not_affect_rest() {
    let file_len = 25;
    let blocks = vec![
      (0..16).collect::<Vec<u8>>(),
      (16..32).collect::<Vec<u8>>(),
      (32..48).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let mut iovecs = IoVecs::bounded(&mut bufs, file_len);

    // 1st advance past the first buffer
    let advance_count = 18;
    iovecs.advance(advance_count);

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half is just the file slice number of bytes after
    // advancing
    let expected_first_half: Vec<_> = blocks
      .iter()
      .flatten()
      .take(file_len)
      .skip(advance_count)
      .collect();
    assert_eq!(first_half, expected_first_half);

    // 2nd advance till the iovecs bound
    let advance_count = file_len - advance_count;
    iovecs.advance(advance_count);

    // the first half of the split should be empty
    let mut first_half = iovecs.as_slice().iter().flat_map(|i| i.as_slice());
    assert!(first_half.next().is_none());
    // same as above
    assert!(iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .next()
      .is_none());

    // restore the second half of the split buffer, which shouldn't be
    // affected by the above advances
    let second_half = iovecs.into_tail();
    // compare the contents of the second half of the split: convert it to
    // a flat vector for easier comparison
    let second_half: Vec<_> =
      second_half.iter().flat_map(|i| i.as_slice()).collect();
    // the length should be the length of the second half the split buffer
    // as well as the remaining block's length
    assert_eq!(second_half.len(), 7 + 16);
    // the expected second half is just the bytes after the file slice number of bytes
    let expected_second_half: Vec<_> =
      blocks.iter().flatten().skip(file_len).collect();
    assert_eq!(second_half, expected_second_half);
  }

  /// Tests that advancing the full write buffer advances only up to the first
  /// half of the split, that is at a buffer boundary, not affecting the second
  /// half.
  #[test]
  fn consuming_first_half_should_not_affect_second_half() {
    let file_len = 32;
    let blocks = vec![
      (0..16).collect::<Vec<u8>>(),
      (16..32).collect::<Vec<u8>>(),
      (32..48).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let mut iovecs = IoVecs::bounded(&mut bufs, file_len);

    // advance past the first two buffers, onto the iovecs bound
    let advance_count = file_len;
    iovecs.advance(advance_count);

    // the first half of the split should be empty
    let mut first_half = iovecs.as_slice().iter().flat_map(|i| i.as_slice());
    assert!(first_half.next().is_none());

    // restore the second half of the split buffer, which shouldn't be
    // affected by the above advance
    let second_half = iovecs.into_tail();
    // compare the contents of the second half of the split: convert it to
    // a flat vector for easier comparison
    let second_half: Vec<_> =
      second_half.iter().flat_map(|i| i.as_slice()).collect();
    // the length should be the length of the second half the split buffer
    // as well as the remaining block's length
    assert_eq!(second_half.len(), 16);
    // the expected second half is just the bytes after the file slice
    // number of bytes
    let expected_second_half: Vec<_> =
      blocks.iter().flatten().skip(file_len).collect();
    assert_eq!(second_half, expected_second_half);
  }

  #[test]
  #[should_panic]
  fn should_panic_advancing_past_end() {
    let file_len = 32;
    let blocks = vec![
      (0..16).collect::<Vec<u8>>(),
      (16..32).collect::<Vec<u8>>(),
      (32..48).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let mut iovecs = IoVecs::bounded(&mut bufs, file_len);

    let advance_count = file_len + 5;
    iovecs.advance(advance_count);
  }

  #[test]
  fn should_advance_into_first_buffer() {
    let mut bufs = vec![vec![0, 1, 2], vec![3, 4, 5]];
    let mut iovecs: Vec<_> =
      bufs.iter_mut().map(|b| IoVec::from_slice(b)).collect();

    // should trim some from the first buffer
    let n = 2;
    let iovecs = advance(&mut iovecs, n);
    let actual: Vec<_> =
      iovecs.iter().flat_map(|b| b.as_slice().to_vec()).collect();
    let expected: Vec<_> = bufs.iter().flatten().skip(n).copied().collect();
    assert_eq!(actual, expected);
  }

  #[test]
  fn should_trim_whole_first_buffer() {
    let mut bufs = vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]];
    let mut iovecs: Vec<_> =
      bufs.iter_mut().map(|b| IoVec::from_slice(b)).collect();

    // should trim entire first buffer
    let n = 3;
    let iovecs = advance(&mut iovecs, n);
    let actual: Vec<_> =
      iovecs.iter().flat_map(|b| b.as_slice().to_vec()).collect();
    let expected: Vec<_> = bufs.iter().flatten().skip(n).copied().collect();
    assert_eq!(actual, expected);
  }

  #[test]
  fn should_advance_into_second_buffer() {
    let mut bufs = vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]];
    let mut iovecs: Vec<_> =
      bufs.iter_mut().map(|b| IoVec::from_slice(b)).collect();

    // should trim entire first buffer and some from second
    let n = 5;
    let iovecs = advance(&mut iovecs, n);
    let actual: Vec<_> =
      iovecs.iter().flat_map(|b| b.as_slice().to_vec()).collect();
    let expected: Vec<_> = bufs.iter().flatten().skip(n).copied().collect();
    assert_eq!(actual, expected);
  }

  #[test]
  fn should_trim_all_buffers() {
    let mut bufs = vec![vec![0, 1, 2], vec![3, 4, 5], vec![6, 7, 8]];
    let mut iovecs: Vec<_> =
      bufs.iter_mut().map(|b| IoVec::from_slice(b)).collect();

    // should trim everything
    let n = 9;
    let iovecs = advance(&mut iovecs, n);
    let mut actual = iovecs.iter().flat_map(|b| b.as_slice().to_vec());
    assert!(actual.next().is_none());
  }

  #[test]
  fn should_advance_one_buffer() {
    let mut bufs = vec![vec![0], vec![1, 2, 3], vec![4, 5, 6]];
    let mut iovecs: Vec<_> =
      bufs.iter_mut().map(|b| IoVec::from_slice(b)).collect();

    let n = 1;
    let iovecs = advance(&mut iovecs, n);
    let actual = iovecs
      .iter()
      .flat_map(|b| b.as_slice().to_vec())
      .collect::<Vec<_>>();
    let expected = bufs
      .iter()
      .skip(1)
      .flat_map(|b| b.clone())
      .collect::<Vec<_>>();
    // //println!("{:?}", actual);

    assert_eq!(actual, expected);
  }

  #[test]
  fn advances_one_buffer_and_tail_should_nice() {
    let file_len = 16;
    let blocks = vec![
      (0..4).collect::<Vec<u8>>(),
      (4..8).collect::<Vec<u8>>(),
      (8..16).collect::<Vec<u8>>(),
    ];

    let mut bufs: Vec<_> =
      blocks.iter().map(|buf| IoVec::from_slice(buf)).collect();
    let mut iovecs = IoVecs::bounded(&mut bufs, file_len);

    // 1st advance past the first buffer
    let advance_count = 4;
    iovecs.advance(advance_count);

    // compare the contents of the first half of the split: convert it
    // to a flat vector for easier comparison
    let first_half: Vec<_> = iovecs
      .as_slice()
      .iter()
      .flat_map(|i| i.as_slice())
      .collect();
    // the expected first half is just the file slice number of bytes after
    // advancing
    let expected_first_half: Vec<_> = blocks
      .iter()
      .flatten()
      .take(file_len)
      .skip(advance_count)
      .collect();

    //println!("{first_half:?}");
    assert_eq!(first_half, expected_first_half);
  }
}
