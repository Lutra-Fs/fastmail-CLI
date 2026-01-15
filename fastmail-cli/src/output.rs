// fastmail-cli/src/output.rs
use serde::Serialize;
use std::fmt;

/// Standard JSON response envelope
#[derive(Debug, Serialize)]
pub struct Response<T> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
}

impl<T> Response<T> {
    pub fn ok(result: T) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
            meta: None,
        }
    }

    pub fn ok_with_meta(result: T, meta: Meta) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
            meta: Some(meta),
        }
    }

    pub fn error(error: ErrorResponse) -> Response<()> {
        Response::<()> {
            ok: false,
            result: None,
            error: Some(error),
            meta: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    #[serde(rename = "type")]
    type_: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retryable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<u64>,
}

impl ErrorResponse {
    pub fn safety_rejected(message: String) -> Self {
        Self {
            type_: "safety_rejected",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn not_found(message: String) -> Self {
        Self {
            type_: "not_found",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn validation_failed(message: String) -> Self {
        Self {
            type_: "validation_failed",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self {
            type_: "rate_limited",
            message: format!("Rate limited. Retry after {}s", retry_after),
            retryable: Some(true),
            retry_after: Some(retry_after),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub remaining: u32,
    pub reset_at: String,
}

/// Exit codes for agent decision making
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success = 0,
    TransientError = 1,
    PermanentError = 2,
    SafetyRejected = 3,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::TransientError => write!(f, "transient_error"),
            Self::PermanentError => write!(f, "permanent_error"),
            Self::SafetyRejected => write!(f, "safety_rejected"),
        }
    }
}

impl ExitCode {
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

// Print response to stdout
pub fn print_response<T: Serialize>(resp: &Response<T>) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(resp)?);
    Ok(())
}
