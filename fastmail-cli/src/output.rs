// fastmail-cli/src/output.rs
use serde::Serialize;
use std::fmt;
use std::io::IsTerminal;

/// Output format option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Auto-detect based on TTY
    Auto,
    /// Force JSON output
    Json,
    /// Force human-readable output
    Human,
}

/// Trait for types that can be formatted for output
pub trait Formattable {
    /// Format as JSON string
    fn to_json(&self) -> String;

    /// Format as human-readable string
    fn to_human(&self) -> String;
}

/// Format output based on the specified format
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

/// Print a styled success message
pub fn print_success(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{} {}\n", console::style("✓").green(), message));
}

/// Print a styled error message
pub fn print_error(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{} {}\n", console::style("Error:").red(), message));
}

/// Print a styled warning message
pub fn print_warning(message: &str) {
    let term = console::Term::stdout();
    let _ = term.write_str(&format!("{} {}\n", console::style("Warning:").yellow(), message));
}

/// Print a styled info/header
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
        let data = TestData { message: "hello".to_string() };
        let result = format_output(&data, OutputFormat::Json);
        assert_eq!(result, r#"{"message":"hello"}"#);
    }

    #[test]
    fn test_format_output_human() {
        let data = TestData { message: "hello".to_string() };
        let result = format_output(&data, OutputFormat::Human);
        assert_eq!(result, "Message: hello");
    }
}
