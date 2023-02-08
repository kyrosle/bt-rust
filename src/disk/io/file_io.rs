//! TODO:  if target_os == linux using preadv and pwritev
//! the relevant module `iovecs`, `pieces`, `file_io`.
use std::{os::windows::prelude::FileExt, sync::Arc};

use crate::{
    blockinfo::CachedBlock,
    error::disk::{ReadError, WriteError},
    iovecs::{IoVec, IoVecs},
    storage_info::FileSlice,
    BLOCK_LEN,
};

use super::file::TorrentFile;

impl TorrentFile {
    /// Writes to file at most the slice length number of bytes of blocks at
    /// teh file slice's offset, using `pwritev`(if in linux), called repeatedly until all
    /// blocks are written to disk.
    ///
    /// It returns the slice of blocks that weren't written to disk. That is,
    /// it returns the second half of `blocks` as through they were split at
    /// the `file_slice.len` offset. If all blocks were written to disk an empty
    /// slice is returned.
    ///
    /// # Important
    ///
    /// Since the system-call may be invoked repeatedly to perform disk IO, this
    /// means that this operation is not guaranteed to be atomic.
    pub fn write<'a>(
        &mut self,
        file_slice: FileSlice,
        blocks: &'a mut [IoVec],
    ) -> Result<&'a mut [IoVec], WriteError> {
        let iovecs = IoVecs::bounded(
            blocks,
            file_slice.len as usize,
        );
        //println!("iovecs: {iovecs:?}");
        // the write buffer cannot be larger than the file slice we want to write to.
        debug_assert!(
            iovecs
                .as_slice()
                .iter()
                .map(|iov| iov.as_slice().len() as u64)
                .sum::<u64>()
                <= file_slice.len
        );

        // IO system-call are not guaranteed to transfer the whole input buffer in
        // one go, so we need to repeat until all bytes have been confirmed to be
        // transferred to dis (or an error occurs)
        // let mut total_write_count = 0;

        //  let write_count = pwritev(
        //     self.handle.as_raw_fd(),
        //     iovecs.as_slice(),
        //     file_slice.offset as i64,
        // )
        // let offset = self
        //     .handle
        //     .seek(io::SeekFrom::Start(file_slice.offset))
        //     .map_err(|e| {
        //         log::warn!(
        //             "File {:?} cannot seek to the offset {} with error {}",
        //             self.info.path,
        //             file_slice.offset,
        //             e
        //         );
        //         WriteError::Io(std::io::Error::last_os_error())
        //     })?;
        // let write_count = self
        //     .handle
        //     .write_all(iovecs.as_u8_vec().as_slice())
        //     .map_err(|e| {
        //         log::warn!("File {:?} write error: {}", self.info.path, e);
        //         WriteError::Io(std::io::Error::last_os_error())
        //     })?;

        // //println!("{}", file_slice.offset + total_write_count);
        //println!(
        //     "write in {:?}",
        //     iovecs.as_u8_vec().as_slice(),
        // );
        self.handle
            .seek_write(
                iovecs.as_u8_vec().as_slice(),
                file_slice.offset,
            )
            .map_err(|e| {
                log::trace!(
                    "File {:?} write error: {}",
                    self.info.path,
                    e
                );
                WriteError::Io(
                    std::io::Error::last_os_error(),
                )
            })?;

        // tally up the total write count
        // total_write_count += write_count as u64;
        // //println!("write: {write_count}");

        // no need to advance write buffers cursor if we're written
        // all of it to file --in that case, we can just split the
        // iovecs and return the second half, consuming the first half
        // if total_write_count == file_slice.len {
        //     break;
        // }

        // advance the buffer cursor in iovecs by the number of bytes
        // transferred
        // iovecs.advance(write_count);

        Ok(iovecs.into_tail())
    }

    /// Reads from file at most the slice length number of bytes of blocks at
    /// the file slice's offset, using `preadv` called repeatedly until all
    /// blocks are read from disk.
    ///
    /// It returns the slice of blocks buffers that weren't filled by the
    /// disk-read. That is, it returns the second half of `block` as though
    /// they were split at the `file_slice.len` offset. If all blocks were read
    /// from disk an empty slice is returned.
    ///
    /// # Important
    ///
    /// Since the system-call may be invoked repeatedly to perform disk IO, this
    /// means that this operation is not guaranteed to be atomic.
    #[allow(clippy::modulo_one)]
    pub fn read(
        &self,
        file_slice: FileSlice,
    ) -> Result<Vec<CachedBlock>, ReadError> {
        // This is simpler than the write implementation as the preadv methods
        // stops reading in from the file if reading EOF. We do need to advance
        // the iovecs read buffer cursor after a read as we may want to read
        // from other files after this one, in which case the cursor should
        // be on the next byte to read to.

        // IO system-call are not guaranteed to transfer the whole input buffer
        // in one go, so we need to repeat until all bytes have been confirmed
        // to be transferred to disk (or an error occurred).

        let mut data =
            vec![0u8; file_slice.len as usize];
        let total_read_count = self
            .handle
            .seek_read(&mut data, file_slice.offset)
            .map_err(|e| {
                log::warn!(
                    "File {:?} read error: {}",
                    self.info.path,
                    e
                );
                ReadError::Io(
                    std::io::Error::last_os_error(),
                )
            })?;

        if total_read_count == 0 {
            return Err(ReadError::MissingData);
        }

        let blocks = data
            .into_iter()
            .fold(
                (Vec::new(), 0),
                |(mut vec, index), x| {
                    if index % BLOCK_LEN == 0 {
                        vec.push(Vec::new());
                    }
                    vec.last_mut().unwrap().push(x);
                    (vec, index + 1)
                },
            )
            .0
            .into_iter()
            .map(Arc::new)
            .collect();

        Ok(blocks)

        // //println!("{}", total_read_count);
        // //println!("{:?}", iovecs);

        // ---
        // In linux using the api `preadv` need to advance the buffer because the vector io system-call
        // may not write all into the buffer in one go, should repeatedly advance until reach the end of buffer.
        //
        // But in window, I have not found any way to use vector io in windows platform,
        // so, I using the standard api `seek_read` which is a one go api.
        // This may inefficient, but maybe I can optimize in future.
        // ---
        // iovecs = advance(iovecs, total_read_count as usize);

        // while !iovecs.is_empty() && (total_read_count as u64) < file_slice.len {
        //     //  let read_count = preadv(
        //     //     self.handle.as_raw_fd(),
        //     //     iovecs,
        //     //     file_slice.offset as i64,
        //     // )
        //     // let read_count =
        //     // self.handle.read_vectored(iovecs).map_err(|e| {
        //     //     log::warn!("File {:?} read error: {}", self.info.path, e);
        //     //     ReadError::Io(std::io::Error::last_os_error())
        //     // })?;

        //     let mut data = vec![];
        //     let read_count = self
        //         .handle
        //         .seek_read(&mut data, file_slice.offset + total_read_count)
        //         .map_err(|e| {
        //             log::trace!("File {:?} read error: {}", self.info.path, e);
        //             ReadError::Io(std::io::Error::last_os_error())
        //         })?;

        //     // if there was nothing to read from file it means we tried to
        //     // read a piece from a portion of a file not yet downloaded or
        //     // otherwise missing.
        //     if read_count == 0 {
        //         return Err(ReadError::MissingData);
        //     }

        //     // tally up the total read count
        //     total_read_count += read_count as u64;

        //     // advance the buffer cursor in iovecs by the number of bytes
        //     // transferred
        //     iovecs = advance(iovecs, read_count);
        // }
    }
}
