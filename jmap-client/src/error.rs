// jmap-client/src/error.rs
use serde::Deserialize;
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

/// JMAP method-level error (RFC 8620 §3.6.2)
#[derive(Debug, Clone, Deserialize)]
pub struct MethodError {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Common JMAP error types
pub mod error_types {
    pub const UNKNOWN_CAPABILITY: &str = "urn:ietf:params:jmap:error:unknownCapability";
    pub const NOT_JSON: &str = "urn:ietf:params:jmap:error:notJSON";
    pub const NOT_REQUEST: &str = "urn:ietf:params:jmap:error:notRequest";
    pub const LIMIT: &str = "urn:ietf:params:jmap:error:limit";

    // Method-level
    pub const SERVER_UNAVAILABLE: &str = "serverUnavailable";
    pub const SERVER_FAIL: &str = "serverFail";
    pub const SERVER_PARTIAL_FAIL: &str = "serverPartialFail";
    pub const UNKNOWN_METHOD: &str = "unknownMethod";
    pub const INVALID_ARGUMENTS: &str = "invalidArguments";
    pub const INVALID_RESULT_REFERENCE: &str = "invalidResultReference";
    pub const FORBIDDEN: &str = "forbidden";
    pub const ACCOUNT_NOT_FOUND: &str = "accountNotFound";
    pub const ACCOUNT_NOT_SUPPORTED_BY_METHOD: &str = "accountNotSupportedByMethod";
    pub const ACCOUNT_READ_ONLY: &str = "accountReadOnly";
}
