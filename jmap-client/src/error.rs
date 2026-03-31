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

/// Structured JMAP error type (RFC 8620 §3.6.2)
#[derive(Debug, Clone, Error)]
pub enum JmapError {
    // RFC 8620 §3.6.2 request-level errors
    #[error("unknown capability: {0}")]
    UnknownCapability(String),

    #[error("not JSON: {0}")]
    NotJson(String),

    #[error("not request: {0}")]
    NotRequest(String),

    #[error("limit exceeded: {0}")]
    Limit(String),

    // RFC 8620 §3.6.2 method-level errors
    #[error("server unavailable")]
    ServerUnavailable,

    #[error("server fail: {description:?}")]
    ServerFail { description: Option<String> },

    #[error("server partial fail")]
    ServerPartialFail,

    #[error("unknown method: {0}")]
    UnknownMethod(String),

    #[error("invalid arguments: {description:?}")]
    InvalidArguments { description: Option<String> },

    #[error("invalid result reference")]
    InvalidResultReference,

    #[error("forbidden")]
    Forbidden,

    #[error("account not found: {0}")]
    AccountNotFound(String),

    #[error("account not supported by method")]
    AccountNotSupportedByMethod,

    #[error("account read only")]
    AccountReadOnly,

    // Catch-all for unknown error types
    #[error("JMAP error {type_}: {description:?}")]
    Unknown {
        type_: String,
        description: Option<String>,
    },
}

impl JmapError {
    /// Parse a JMAP error response (the args field of an error invocation) into a typed variant.
    pub fn from_value(args: &serde_json::Value) -> Self {
        let type_ = args
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        match type_ {
            error_types::UNKNOWN_CAPABILITY => {
                Self::UnknownCapability(description.unwrap_or_default())
            }
            error_types::NOT_JSON => Self::NotJson(description.unwrap_or_default()),
            error_types::NOT_REQUEST => Self::NotRequest(description.unwrap_or_default()),
            error_types::LIMIT => Self::Limit(description.unwrap_or_default()),
            error_types::SERVER_UNAVAILABLE => Self::ServerUnavailable,
            error_types::SERVER_FAIL => Self::ServerFail { description },
            error_types::SERVER_PARTIAL_FAIL => Self::ServerPartialFail,
            error_types::UNKNOWN_METHOD => Self::UnknownMethod(description.unwrap_or_default()),
            error_types::INVALID_ARGUMENTS => Self::InvalidArguments { description },
            error_types::INVALID_RESULT_REFERENCE => Self::InvalidResultReference,
            error_types::FORBIDDEN => Self::Forbidden,
            error_types::ACCOUNT_NOT_FOUND => {
                Self::AccountNotFound(description.unwrap_or_default())
            }
            error_types::ACCOUNT_NOT_SUPPORTED_BY_METHOD => Self::AccountNotSupportedByMethod,
            error_types::ACCOUNT_READ_ONLY => Self::AccountReadOnly,
            _ => Self::Unknown {
                type_: type_.to_string(),
                description,
            },
        }
    }
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
