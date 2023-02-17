use std::{
  fs::OpenOptions, os::windows::prelude::FileExt,
};

pub fn main() {
  // let mut fs = tempfile().unwrap();
  let fs = OpenOptions::new()
    .read(true)
    .write(true)
    .open(r#"fixtures/bytes.torrent"#)
    .unwrap();
  let msg = "1234567890"
    .chars()
    .map(|c| c as u8)
    .collect::<Vec<u8>>();
  let mut offset = 0;
  offset +=
    fs.seek_write(&msg, offset).unwrap() as u64;
  offset +=
    fs.seek_write(&msg, offset).unwrap() as u64;

  let mut buffer = Vec::new();
  buffer.resize(offset as usize, 0u8);
  // let mut buffer = [0u8; 10];

  // offset = 0;
  // fs.seek(SeekFrom::Start(0)).unwrap();
  // offset = fs.take(msg.len() as u64).read_to_end(&mut buffer).unwrap() as u64;
  // offset = fs.take(msg.len() as u64).read_to_end(&mut buffer).unwrap() as u64;

  fs.sync_data().unwrap();

  fs.seek_read(&mut buffer, 0).unwrap();
  //println!("{buffer:?}");
}
