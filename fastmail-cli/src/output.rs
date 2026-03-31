// fastmail-cli/src/output.rs
use serde::Serialize;
use std::fmt;
use std::io::IsTerminal;

/// Output format option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum OutputFormat {
    /// Auto-detect based on TTY
    Auto,
    /// Force JSON output
    Json,
    /// Force human-readable output
    Human,
}

/// Trait for types that can be formatted for output
#[allow(dead_code)]
pub trait Formattable {
    /// Format as JSON string
    fn to_json(&self) -> String;

    /// Format as human-readable string
    fn to_human(&self) -> String;
}

/// Format output based on the specified format
#[allow(dead_code)]
pub fn format_output<T: Formattable>(data: &T, format: OutputFormat) -> String {
    match format {
        OutputFormat::Json => data.to_json(),
        OutputFormat::Human => data.to_human(),
        OutputFormat::Auto => {
            if std::io::stdout().is_terminal() {
                data.to_human()
            } else {
                data.to_json()
            }
        }
    }
}

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

    #[allow(dead_code)]
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
#[allow(dead_code)]
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

/// Print a styled success message
#[allow(dead_code)]
pub fn print_success(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{} {}\n", console::style("✓").green(), message));
}

/// Print a styled error message
#[allow(dead_code)]
pub fn print_error(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{} {}\n", console::style("Error:").red(), message));
}

/// Print a styled warning message
#[allow(dead_code)]
pub fn print_warning(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!(
        "{} {}\n",
        console::style("Warning:").yellow(),
        message
    ));
}

/// Print a styled info/header
#[allow(dead_code)]
pub fn print_header(key: &str, value: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{}: {}\n", console::style(key).bold(), value));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestData {
        message: String,
    }

    impl Formattable for TestData {
        fn to_json(&self) -> String {
            format!(r#"{{"message":"{}"}}"#, self.message)
        }

        fn to_human(&self) -> String {
            format!("Message: {}", self.message)
        }
    }

    #[test]
    fn test_format_output_json() {
        let data = TestData {
            message: "hello".to_string(),
        };
        let result = format_output(&data, OutputFormat::Json);
        assert_eq!(result, r#"{"message":"hello"}"#);
    }

    #[test]
    fn test_format_output_human() {
        let data = TestData {
            message: "hello".to_string(),
        };
        let result = format_output(&data, OutputFormat::Human);
        assert_eq!(result, "Message: hello");
    }

    #[test]
    fn test_response_ok_serialization() {
        let resp = Response::ok(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_response_ok_with_meta_serialization() {
        let meta = Meta {
            rate_limit: Some(RateLimitInfo {
                remaining: 42,
                reset_at: "2025-01-01T00:00:00Z".to_string(),
            }),
            dry_run: Some(true),
            operation_id: Some("op-123".to_string()),
        };
        let resp = Response::ok_with_meta(vec!["x".to_string()], meta);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"ok\":true"));
        assert!(json.contains("\"rate_limit\""));
        assert!(json.contains("\"dry_run\":true"));
        assert!(json.contains("\"operation_id\":\"op-123\""));
    }

    #[test]
    fn test_response_error_serialization() {
        let resp = Response::<()>::error(ErrorResponse::safety_rejected("test".to_string()));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"ok\":false"));
        assert!(json.contains("safety_rejected"));
        assert!(json.contains("\"retryable\":false"));
    }

    #[test]
    fn test_response_error_skips_null_fields() {
        let resp = Response::<()>::error(ErrorResponse::not_found("gone".to_string()));
        let json = serde_json::to_string(&resp).unwrap();
        // result and meta should be absent, not null
        assert!(!json.contains("\"result\""));
        assert!(!json.contains("\"meta\""));
        assert!(!json.contains("\"retry_after\""));
    }

    #[test]
    fn test_error_response_safety_rejected() {
        let err = ErrorResponse::safety_rejected("bad command".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"type\":\"safety_rejected\""));
        assert!(json.contains("\"message\":\"bad command\""));
        assert!(json.contains("\"retryable\":false"));
    }

    #[test]
    fn test_error_response_not_found() {
        let err = ErrorResponse::not_found("no mailbox".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"type\":\"not_found\""));
    }

    #[test]
    fn test_error_response_validation_failed() {
        let err = ErrorResponse::validation_failed("bad input".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"type\":\"validation_failed\""));
    }

    #[test]
    fn test_error_response_rate_limited() {
        let err = ErrorResponse::rate_limited(30);
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"type\":\"rate_limited\""));
        assert!(json.contains("\"retryable\":true"));
        assert!(json.contains("\"retry_after\":30"));
        assert!(json.contains("30s"));
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(ExitCode::Success.code(), 0);
        assert_eq!(ExitCode::TransientError.code(), 1);
        assert_eq!(ExitCode::PermanentError.code(), 2);
        assert_eq!(ExitCode::SafetyRejected.code(), 3);
    }

    #[test]
    fn test_exit_code_display() {
        assert_eq!(ExitCode::Success.to_string(), "success");
        assert_eq!(ExitCode::TransientError.to_string(), "transient_error");
        assert_eq!(ExitCode::PermanentError.to_string(), "permanent_error");
        assert_eq!(ExitCode::SafetyRejected.to_string(), "safety_rejected");
    }
}
