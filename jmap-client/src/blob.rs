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
