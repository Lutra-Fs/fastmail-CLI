// jmap-client/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("Blob capability not supported by server")]
    CapabilityNotSupported,

    #[error("Unknown data type: {0}")]
    UnknownDataType(String),

    #[error("Blob size exceeds maximum: {size} > {max_size}")]
    SizeExceeded { size: u64, max_size: u64 },

    #[error("Invalid data source range: offset={offset}, length={length}")]
    InvalidRange { offset: u64, length: u64 },

    #[error("Blob not found: {0}")]
    NotFound(String),

    #[error("Encoding problem: blob data is not valid UTF-8")]
    EncodingProblem,

    #[error("Blob data was truncated: requested range extends beyond blob")]
    Truncated,

    #[error("Invalid base64 encoding: {0}")]
    InvalidBase64(String),
}
