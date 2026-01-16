// jmap-client/src/blob.rs
use crate::error::BlobError;
use crate::types::DataSourceObject;
use anyhow::Result;

/// Encode bytes as base64
pub fn encode_base64(data: &[u8]) -> String {
    use base64::prelude::*;
    BASE64_STANDARD.encode(data)
}

/// Decode base64 string to bytes
pub fn decode_base64(s: &str) -> Result<Vec<u8>> {
    use base64::prelude::*;
    BASE64_STANDARD
        .decode(s)
        .map_err(|e| BlobError::InvalidBase64(e.to_string()).into())
}

/// Create DataSourceObject from raw bytes
pub fn data_source_from_bytes(bytes: &[u8]) -> DataSourceObject {
    DataSourceObject::AsBase64 {
        data_as_base64: encode_base64(bytes),
    }
}

/// Create DataSourceObject from text
pub fn data_source_from_text(text: &str) -> DataSourceObject {
    DataSourceObject::AsText {
        data_as_text: text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_base64() {
        assert_eq!(encode_base64(b"hello"), "aGVsbG8=");
        assert_eq!(encode_base64(b""), "");
    }

    #[test]
    fn test_decode_base64() {
        assert_eq!(decode_base64("aGVsbG8=").unwrap().as_slice(), b"hello");
        assert_eq!(decode_base64("").unwrap().as_slice(), b"");
    }

    #[test]
    fn test_decode_base64_invalid() {
        assert!(decode_base64("not-valid-base64!!!").is_err());
    }

    #[test]
    fn test_data_source_from_bytes() {
        let ds = data_source_from_bytes(b"hello");
        match ds {
            DataSourceObject::AsBase64 { data_as_base64 } => {
                assert_eq!(data_as_base64, "aGVsbG8=");
            }
            _ => panic!("Expected AsBase64 variant"),
        }
    }

    #[test]
    fn test_data_source_from_text() {
        let ds = data_source_from_text("hello world");
        match ds {
            DataSourceObject::AsText { data_as_text } => {
                assert_eq!(data_as_text, "hello world");
            }
            _ => panic!("Expected AsText variant"),
        }
    }
}
