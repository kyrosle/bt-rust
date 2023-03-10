pub mod handshake;
pub mod message;
pub mod peercodec;

#[cfg(test)]
mod tests {
  use bytes::BufMut;
  use bytes::{Bytes, BytesMut};
  use tokio_util::codec::Decoder;
  use tokio_util::codec::Encoder;

  use super::handshake::*;
  use super::message::*;
  use super::peercodec::*;
  use crate::blockinfo::BlockInfo;
  use crate::{Bitfield, BLOCK_LEN};

  /// Tests a stream of arbitrary messages to ensure that not only do they
  /// encode and then decode correctly (like the individual test cases
  /// ascertain), but that the buffer cursor is properly advanced by the codec
  /// implementation in both cases.
  #[test]
  fn test_message_stream() {
    let (handshake, encoded_handshake) = make_handshake();
    let msgs = [
      make_choke(),
      make_unchoke(),
      make_keep_alive(),
      make_interested(),
      make_not_interested(),
      make_bitfield(),
      make_have(),
      make_request(),
      make_block(),
      make_block(),
      make_keep_alive(),
      make_interested(),
      make_cancel(),
      make_block(),
      make_not_interested(),
      make_choke(),
      make_choke(),
    ];

    // create byte stream of all above messages
    let msgs_len = msgs.iter().fold(0, |acc, (_, encoded)| acc + encoded.len());
    let mut read_buf = BytesMut::with_capacity(msgs_len);
    read_buf.extend_from_slice(&encoded_handshake);
    for (_, encoded) in &msgs {
      read_buf.extend_from_slice(encoded);
    }

    // decode messages one by one from the byte stream in the same order as
    // they were encoded, starting with the handshake
    let decoded_handshake = HandshakeCodec.decode(&mut read_buf).unwrap();
    assert_eq!(decoded_handshake, Some(handshake));
    for (msg, _) in &msgs {
      let decoded_msg = PeerCodec.decode(&mut read_buf).unwrap();
      assert_eq!(decoded_msg.unwrap(), *msg);
    }
  }

  // This test attempts to simulate a closer to real world use case than
  // `test_test_message_stream`, by progresively loading up the codec's read
  // buffer with the encoded message bytes, asserting that messages are
  // decoded correctly even if their bytes arrives in different chunks.
  //
  // This is a regression test in that there used to be a bug that failed to
  // parse block messages (the largest message type) if the full message
  // couldn't be received (as is often the case).
  #[test]
  fn test_chunked_message_stream() {
    let mut read_buf = BytesMut::new();

    // start with the handshake by adding only the first half of it to the
    // buffer
    let (handshake, encoded_handshake) = make_handshake();
    let handshake_split_pos = encoded_handshake.len() / 2;
    read_buf.extend_from_slice(&encoded_handshake[0..handshake_split_pos]);

    // can't decode the handshake without the full message
    assert!(HandshakeCodec.decode(&mut read_buf).unwrap().is_none());

    // the handshake should successfully decode with the second half added
    read_buf.extend_from_slice(&encoded_handshake[handshake_split_pos..]);
    let decoded_handshake = HandshakeCodec.decode(&mut read_buf).unwrap();
    assert_eq!(decoded_handshake, Some(handshake));

    let msgs = [
      make_choke(),
      make_unchoke(),
      make_interested(),
      make_not_interested(),
      make_bitfield(),
      make_have(),
      make_request(),
      make_block(),
      make_block(),
      make_interested(),
      make_cancel(),
      make_block(),
      make_not_interested(),
      make_choke(),
      make_choke(),
    ];

    // go through all above messages and do the same procedure as with the
    // handshake: add the first half, fail to decode, add the second half,
    // decode successfully
    for (msg, encoded) in &msgs {
      // add the first half of the message
      let split_pos = encoded.len() / 2;
      read_buf.extend_from_slice(&encoded[0..split_pos]);
      // fail to decode
      assert!(PeerCodec.decode(&mut read_buf).unwrap().is_none());
      // add the second half
      read_buf.extend_from_slice(&encoded[split_pos..]);
      let decoded_msg = PeerCodec.decode(&mut read_buf).unwrap();
      assert_eq!(decoded_msg.unwrap(), *msg);
    }
  }

  /// Tests the encoding and subsequent decoding of a valid handshake.
  #[test]
  fn test_handshake_codec() {
    let (handshake, expected_encoded) = make_handshake();

    // encode handshake
    let mut encoded = BytesMut::with_capacity(expected_encoded.len());
    HandshakeCodec.encode(handshake, &mut encoded).unwrap();
    assert_eq!(encoded, expected_encoded);

    // don't decode handshake if there aren't enough bytes in source buffer
    let mut partial_encoded = encoded[0..30].into();
    let decoded = HandshakeCodec.decode(&mut partial_encoded).unwrap();
    assert_eq!(decoded, None);

    // decode same handshake
    let decoded = HandshakeCodec.decode(&mut encoded).unwrap();
    assert_eq!(decoded, Some(handshake));
  }

  /// Tests that the decoding of various invalid handshake messages results in
  /// an error.
  #[test]
  fn test_invalid_handshake_decoding() {
    // try to decode a handshake with an invalid protocol string
    let mut invalid_encoded = {
      let prot = "not the BitTorrent protocol";
      // these buffer values don't matter here as we're only expecting
      // invalid encodings
      let reserved = [0; 8];
      let info_hash = [0; 20];
      let peer_id = [0; 20];

      let buf_len = prot.len() + 49;
      let mut buf = BytesMut::with_capacity(buf_len);
      // the message length prefix is not actually included in the value
      let prot_len = prot.len() as u8;
      buf.put_u8(prot_len);
      buf.extend_from_slice(prot.as_bytes());
      buf.extend_from_slice(&reserved);
      buf.extend_from_slice(&info_hash);
      buf.extend_from_slice(&peer_id);
      buf
    };
    let result = HandshakeCodec.decode(&mut invalid_encoded);
    assert!(result.is_err());
  }

  // Returns a `Handshake` and its expected encoded variant.
  fn make_handshake() -> (Handshake, Bytes) {
    // protocol string
    let mut prot = [0; 19];
    prot.copy_from_slice(PROTOCOL_STRING.as_bytes());

    // the reserved field is all zeros for now as we don't use extensions
    // yet so we're not testing it
    let reserved = [0; 8];

    // this is not a valid info hash but it doesn't matter for the purposes
    // of this test
    const INFO_HASH: &str = "da39a3ee5e6b4b0d3255";
    let mut info_hash = [0; 20];
    info_hash.copy_from_slice(INFO_HASH.as_bytes());

    const PEER_ID: &str = "cbt-2020-03-03-00000";
    let mut peer_id = [0; 20];
    peer_id.copy_from_slice(PEER_ID.as_bytes());

    let handshake = Handshake {
      prot,
      reserved,
      info_hash,
      peer_id,
    };

    // TODO: consider using hard coded bye array for expected value rather
    // than building up result (as that's what the encoder does too and we
    // need to test that it does it correctly)
    let encoded = {
      let buf_len = 68;
      let mut buf = Vec::with_capacity(buf_len);
      // the message length prefix is not actually included in the value
      let prot_len = prot.len() as u8;
      buf.push(prot_len);
      buf.extend_from_slice(&prot);
      buf.extend_from_slice(&reserved);
      buf.extend_from_slice(&info_hash);
      buf.extend_from_slice(&peer_id);
      buf
    };

    (handshake, encoded.into())
  }

  /// Tests the encoding and subsequent decoding of a valid 'choke' message.
  #[test]
  fn test_keep_alive_codec() {
    let (msg, expected_encoded) = make_keep_alive();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'choke' message.
  #[test]
  fn test_choke_codec() {
    let (msg, expected_encoded) = make_choke();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'unchoke' message.
  #[test]
  fn test_unchoke_codec() {
    let (msg, expected_encoded) = make_unchoke();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'interested'
  /// message.
  #[test]
  fn test_interested_codec() {
    let (msg, expected_encoded) = make_interested();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'not interested'
  /// message.
  #[test]
  fn test_not_interested_codec() {
    let (msg, expected_encoded) = make_not_interested();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'bitfield' message.
  #[test]
  fn test_bitfield_codec() {
    let (msg, expected_encoded) = make_bitfield();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'have' message.
  #[test]
  fn test_have_codec() {
    let (msg, expected_encoded) = make_have();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'request' message.
  #[test]
  fn test_request_codec() {
    let (msg, expected_encoded) = make_request();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'block' message.
  #[test]
  fn test_block_codec() {
    let (msg, expected_encoded) = make_block();
    assert_message_codec(msg, expected_encoded);
  }

  /// Tests the encoding and subsequent decoding of a valid 'cancel' message.
  #[test]
  fn test_cancel_codec() {
    let (msg, expected_encoded) = make_cancel();
    assert_message_codec(msg, expected_encoded);
  }

  /// Helper function that asserts that a message is encoded and subsequently
  /// decoded correctly.
  fn assert_message_codec(msg: Message, expected_encoded: Bytes) {
    // encode message
    let mut encoded = BytesMut::with_capacity(expected_encoded.len());
    PeerCodec.encode(msg.clone(), &mut encoded).unwrap();
    assert_eq!(encoded, expected_encoded);

    // don't decode message if there aren't enough bytes in source buffer
    let mut partial_encoded = encoded[0..encoded.len() - 1].into();
    let decoded = PeerCodec.decode(&mut partial_encoded).unwrap();
    assert_eq!(decoded, None);

    // decode same message
    let decoded = PeerCodec.decode(&mut encoded).unwrap();
    assert_eq!(decoded, Some(msg));
  }

  fn make_keep_alive() -> (Message, Bytes) {
    (Message::KeepAlive, Bytes::from_static(&[0; 4]))
  }

  // Returns `Choke` and its expected encoded variant.
  fn make_choke() -> (Message, Bytes) {
    (
      Message::Choke,
      make_empty_msg_encoded_payload(MessageId::Choke),
    )
  }

  /// Returns `Unchoke` and its expected encoded variant.
  fn make_unchoke() -> (Message, Bytes) {
    (
      Message::Unchoke,
      make_empty_msg_encoded_payload(MessageId::Unchoke),
    )
  }

  /// Returns `Interested` and its expected encoded variant.
  fn make_interested() -> (Message, Bytes) {
    (
      Message::Interested,
      make_empty_msg_encoded_payload(MessageId::Interested),
    )
  }

  /// Returns `NotInterested` and its expected encoded variant.
  fn make_not_interested() -> (Message, Bytes) {
    (
      Message::NotInterested,
      make_empty_msg_encoded_payload(MessageId::NotInterested),
    )
  }

  /// Helper used to create 'choke', 'unchoke', 'interested', and 'not
  /// interested' encoded messages that all have the same format.
  fn make_empty_msg_encoded_payload(id: MessageId) -> Bytes {
    // 1 byte message id
    let msg_len = 1;
    // 4 byte message length prefix and message length
    let buf_len = 4 + msg_len as usize;
    let mut buf = BytesMut::with_capacity(buf_len);
    buf.put_u32(msg_len);
    buf.put_u8(id as u8);
    buf.into()
  }

  /// Returns `Bitfield` and its expected encoded variant.
  fn make_bitfield() -> (Message, Bytes) {
    let bitfield = Bitfield::from_vec(vec![0b11001001, 0b10000011, 0b11111011]);
    let encoded = {
      // 1 byte message id and n byte f bitfield
      //
      // NOTE: `bitfield.len()` returns the number of _bits_
      let msg_len = 1 + bitfield.len() / 8;
      // 4 byte message length prefix and message length
      let buf_len = 4 + msg_len;
      let mut buf = BytesMut::with_capacity(buf_len);
      buf.put_u32(msg_len as u32);
      buf.put_u8(MessageId::Bitfield as u8);
      buf.extend_from_slice(bitfield.as_raw_slice());
      buf
    };
    let msg = Message::Bitfield(bitfield);
    (msg, encoded.into())
  }

  /// Returns `Have` and its expected encoded variant.
  fn make_have() -> (Message, Bytes) {
    let piece_index = 42;
    let msg = Message::Have { piece_index };
    let encoded = {
      // 1 byte message id and 4 byte piece index
      let msg_len = 1 + 4;
      // 4 byte message length prefix and message length
      let buf_len = 4 + msg_len;
      let mut buf = BytesMut::with_capacity(buf_len);
      buf.put_u32(msg_len as u32);
      buf.put_u8(MessageId::Have as u8);
      // ok to unwrap, only used in tests
      buf.put_u32(piece_index.try_into().unwrap());
      buf
    };
    (msg, encoded.into())
  }

  /// Returns `Request` and its expected encoded variant.
  fn make_request() -> (Message, Bytes) {
    let piece_index = 42;
    let offset = 0x4000;
    let len = BLOCK_LEN;
    let msg = Message::Request(BlockInfo {
      piece_index,
      offset,
      len,
    });
    let encoded = make_block_info_encoded_msg_payload(
      MessageId::Request,
      piece_index,
      offset,
      len,
    );
    (msg, encoded)
  }

  /// Returns `Block` and its expected encoded variant.
  fn make_block() -> (Message, Bytes) {
    let piece_index = 42;
    let offset = 0x4000;
    let data = vec![0; 0x4000];
    // TODO: fill the block with random values
    let encoded = {
      // 1 byte message id, 4 byte piece index, 4 byte offset, and n byte
      // block
      let msg_len = 1 + 4 + 4 + data.len();
      // 4 byte message length prefix and message length
      let buf_len = 4 + msg_len;
      let mut buf = BytesMut::with_capacity(buf_len);
      buf.put_u32(msg_len as u32);
      buf.put_u8(MessageId::Block as u8);
      // ok to unwrap, only used in tests
      buf.put_u32(piece_index.try_into().unwrap());
      buf.put_u32(offset);
      buf.extend_from_slice(&data);
      buf
    };
    let msg = Message::Block {
      piece_index,
      offset,
      data: data.into(),
    };
    (msg, encoded.into())
  }

  /// Returns `Cancel` and its expected encoded variant.
  fn make_cancel() -> (Message, Bytes) {
    let piece_index = 42;
    let offset = 0x4000;
    let len = BLOCK_LEN;
    let msg = Message::Cancel(BlockInfo {
      piece_index,
      offset,
      len,
    });
    let encoded = make_block_info_encoded_msg_payload(
      MessageId::Cancel,
      piece_index,
      offset,
      len,
    );
    (msg, encoded)
  }

  /// Helper used to create 'request' and 'cancel' encoded messages that have
  /// the same format.
  fn make_block_info_encoded_msg_payload(
    id: MessageId,
    piece_index: usize,
    offset: u32,
    len: u32,
  ) -> Bytes {
    // 1 byte message id, 4 byte piece index, 4 byte offset, 4 byte
    // length
    let msg_len = 1 + 4 + 4 + 4;
    // 4 byte message length prefix and message length
    let buf_len = 4 + msg_len as usize;
    let mut buf = BytesMut::with_capacity(buf_len);
    buf.put_u32(msg_len);
    buf.put_u8(id as u8);
    // ok to unwrap, only used in tests
    buf.put_u32(piece_index.try_into().unwrap());
    buf.put_u32(offset);
    buf.put_u32(len);
    buf.into()
  }
}
