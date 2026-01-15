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

    /// GET request for session (default implementation uses POST)
    async fn get(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        self.post_json(url, body).await
    }
}

#[cfg(feature = "reqwest")]
pub mod reqwest;

#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestClient;
