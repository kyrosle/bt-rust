use std::{
  fs::File,
  io::{IoSlice, IoSliceMut, Read, Write},
};

fn main() -> std::io::Result<()> {
  let data1 = [1; 8];
  let data2 = [15; 8];
  let io_slice1 = IoSlice::new(&data1);
  let io_slice2 = IoSlice::new(&data2);

  let mut buffer = File::create("foo.txt")?;

  // Writes some prefix of the byte string, not necessarily all of it.
  let result = buffer.write_vectored(&[io_slice1, io_slice2])?;

  println!("written {result} bytes");
  buffer.flush()?;
  drop(buffer);

  let mut data1 = [0; 8];
  let mut data2 = [0; 8];
  let mut buffer = File::open("foo.txt")?;
  let io_mut_slice1 = IoSliceMut::new(&mut data1);
  let io_mut_slice2 = IoSliceMut::new(&mut data2);
  // buffer.read_to_end(&mut buffer_s)?;
  let result = buffer.read_vectored(&mut [io_mut_slice1, io_mut_slice2])?;
  println!("read {result} bytes");
  println!("{data1:?}");
  println!("{data2:?}");
  Ok(())
}
