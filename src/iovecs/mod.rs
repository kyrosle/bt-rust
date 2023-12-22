//! This crate provides a helper type for a slice of [`IoVec`]s (in linux) /
//! [`IoSlice`]s (in windows), for zero-copy functionality to bound iovecs by a
//! byte count and to advance teh buffer cursor after partial vectored IO.
//!
//! # Bounding input buffers
//!
//! This is most useful when writing to a portion of a file while ensuring
//! that the file is not extended if the input buffers are larger than the
//! length of the file slice, which in the torrent scenario may occur if the  
//! file is not a multiple of the block size.
//!
//! If the total size of the buffers exceeds the max length, the buffers are
//! split such that the first half returned is the portion of the buffers
//! that can be written to the file, while the second half is the remainder
//! of the buffer, to be used later. If the size of the total size of the
//! buffers is smaller than or equal to the slice, this is essentially a noop.
//!
//! In reality, the situation here is more complex than just splitting
//! buffer in half, but this is taken care of by the [`IoVecs`] implementation.
//!
//! However, the abstraction leaks through because the iovec at which the
//! buffers were split may be shrunk to such a size as would enable all
//! buffers to stay within the file slice length. This can be restored using
//! [`IoVecs::into_tail`], but until this is called, the original buffers
//! cannot be used, which is enforced by the borrow checker.
//!
//! # Advancing the write cursor
//!
//! IO system-call generally don't guarantee writing or filling input buffers
//! in one system call. This is why these APIs always return the number of bytes
//! transferred, so that calling the [`IoVecs::advance`] method, which takes
//! offsets the start of the slices by some number of bytes.
//!
//! # Example
//!
//! What follows is a complete example making use of both above mentioned
//! API features.
//!
//! Visualized, this looks like the following:
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
//! In this example,
//! the first half of the split would be [0, 25),
//! the second half would be [25, 32).

// FIXME: after adapting the linux iovec, than enable this feature, or using `iovec` in linux and using `wasbuf` in window.
// #[cfg(any(target_os = "linux", target_os = "macos"))]
// pub use nix::sys::uio::IoVec;

#[cfg(target_os = "windows")]
pub mod iovec_unit;
#[cfg(target_os = "windows")]
pub use iovec_unit::IoVec;

pub mod test;

#[allow(clippy::module_inception)]
pub mod iovecs;
pub use iovecs::*;
