// jmap-client/src/http/mod.rs
use async_trait::async_trait;

/// Error from HTTP request
#[derive(Debug, Clone)]
pub struct HttpError {
    pub status: Option<u16>,
    pub message: String,
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status) = self.status {
            write!(f, "HTTP error {}: {}", status, self.message)
        } else {
            write!(f, "HTTP error: {}", self.message)
        }
    }
}

impl std::error::Error for HttpError {}

/// Generic HTTP client trait - users can implement their own
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// POST JSON data to URL, return response bytes
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError>;

    /// POST binary data to URL with custom Content-Type, return response bytes
    async fn post_binary(&self, url: &str, data: Vec<u8>, content_type: &str) -> Result<Vec<u8>, HttpError> {
        // Default implementation: override in actual client
        Err(HttpError {
            status: None,
            message: "post_binary not implemented".to_string(),
        })
    }

    /// GET request for session (default implementation uses POST)
    async fn get(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        self.post_json(url, body).await
    }
}

#[cfg(feature = "reqwest")]
pub mod reqwest;

#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestClient;
