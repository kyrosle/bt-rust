/// A Wrapper of `Vec<u8>`, which is planing to redesigned as a zero-copy structure.
///
/// In windows, it is hard to implement a vector io, such that like `scatter` and `gather`,
/// But in linux(such as android, unix platforms), it is available to use `writev` or `readv`,
/// which will make reading from disk or writing to disk more faster.(reducing the time to request
/// system api).
#[derive(Debug)]
pub struct IoVec(Vec<u8>);

impl IoVec {
  /// Create a new IoVec structure, accepting a u8 array slice.
  pub fn from_slice(value: &[u8]) -> Self {
    IoVec(value.to_vec())
  }
  pub fn from_slice_mut(value: &mut [u8]) -> Self {
    IoVec(value.to_vec())
  }
  /// Create a new IoVec structure, take the ownership of the u8 dynamical array.
  pub fn from_vec(value: Vec<u8>) -> Self {
    IoVec(value)
  }
  /// Return a immutable reference to the holding u8 dynamical array.
  pub fn as_slice(&self) -> &[u8] {
    self.0.as_slice()
  }
  /// Return a mutable reference to the holding u8 dynamical array.
  pub fn as_mut_slice(&mut self) -> &mut [u8] {
    self.0.as_mut_slice()
  }
  // no used - planning to remove.
  // pub fn modify<'a>(
  //   &mut self,
  //   value: &'a [u8],
  // ) -> Result<(), &'a [u8]> {
  //   let self_len = self.0.len();
  //   let value_len = value.len();

  //   match self_len.cmp(&value_len) {
  //     std::cmp::Ordering::Equal => {
  //       self.0 = value.to_vec()
  //     }
  //     _ => return Err(value),
  //   }

  //   Ok(())
  // }
  // pub fn len(&self) -> usize {
  //   self.0.len()
  // }
  // pub fn is_empty(&self) -> bool {
  //   self.0.is_empty()
  // }
}
