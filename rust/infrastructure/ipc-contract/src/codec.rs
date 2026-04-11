//! IPC Frame Codec
//!
//! Length-prefixed framing: [4-byte BE length][JSON payload]

use bytes::{Buf, BufMut, BytesMut};
use serde_json::Value;
use tokio_util::codec::{Decoder, Encoder};

use crate::error::IPCError;

/// Maximum frame size (16MB)
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Length-prefixed frame codec
#[derive(Debug, Clone)]
pub struct FrameCodec;

impl FrameCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for FrameCodec {
    type Item = String;
    type Error = IPCError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 4 bytes for length prefix
        if src.len() < 4 {
            return Ok(None);
        }

        // Read length prefix (big endian u32)
        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&src[0..4]);
        let len = u32::from_be_bytes(len_bytes) as usize;

        // Validate length
        if len > MAX_FRAME_SIZE {
            return Err(IPCError::ParseError(format!(
                "Frame size {} exceeds maximum {}",
                len, MAX_FRAME_SIZE
            )));
        }

        // Wait for complete frame
        if src.len() < 4 + len {
            src.reserve(4 + len - src.len());
            return Ok(None);
        }

        // Extract frame data
        let frame_data = src[4..4 + len].to_vec();
        src.advance(4 + len);

        // Parse as UTF-8 string
        String::from_utf8(frame_data)
            .map_err(|e| IPCError::ParseError(format!("Invalid UTF-8 in frame: {}", e)))
            .map(Some)
    }
}

impl Encoder<String> for FrameCodec {
    type Error = IPCError;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let data = item.as_bytes();
        let len = data.len();

        if len > MAX_FRAME_SIZE {
            return Err(IPCError::ParseError(format!(
                "Frame size {} exceeds maximum {}",
                len, MAX_FRAME_SIZE
            )));
        }

        // Write length prefix (big endian u32)
        dst.put_u32(len as u32);

        // Write frame data
        dst.put_slice(data);

        Ok(())
    }
}

impl Encoder<Value> for FrameCodec {
    type Error = IPCError;

    fn encode(&mut self, item: Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json = serde_json::to_string(&item)
            .map_err(|e| IPCError::ParseError(format!("JSON serialize error: {}", e)))?;
        self.encode(json, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let mut codec = FrameCodec::new();
        let mut buffer = BytesMut::new();

        let original = r#"{"method":"test","params":{}}"#;
        codec.encode(original.to_string(), &mut buffer).unwrap();

        let decoded = codec.decode(&mut buffer).unwrap().unwrap();
        assert_eq!(original, decoded);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_multiple_frames() {
        let mut codec = FrameCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode("first".to_string(), &mut buffer).unwrap();
        codec.encode("second".to_string(), &mut buffer).unwrap();

        assert_eq!(codec.decode(&mut buffer).unwrap().unwrap(), "first");
        assert_eq!(codec.decode(&mut buffer).unwrap().unwrap(), "second");
        assert!(codec.decode(&mut buffer).unwrap().is_none());
    }

    #[test]
    fn test_incomplete_frame() {
        let mut codec = FrameCodec::new();
        let mut buffer = BytesMut::new();

        buffer.extend_from_slice(&[0, 0, 0, 5]); // length = 5
        buffer.extend_from_slice(b"hel"); // only 3 bytes

        assert!(codec.decode(&mut buffer).unwrap().is_none());

        buffer.extend_from_slice(b"lo"); // now complete
        assert_eq!(codec.decode(&mut buffer).unwrap().unwrap(), "hello");
    }

    #[test]
    fn test_json_value_encode() {
        let mut codec = FrameCodec::new();
        let mut buffer = BytesMut::new();

        let value = serde_json::json!({"test": 123});
        codec.encode(value.clone(), &mut buffer).unwrap();

        let decoded = codec.decode(&mut buffer).unwrap().unwrap();
        let parsed: Value = serde_json::from_str(&decoded).unwrap();
        assert_eq!(value, parsed);
    }
}
