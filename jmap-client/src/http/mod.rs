// jmap-client/src/http/mod.rs
use async_trait::async_trait;

/// Error from HTTP request
#[derive(Debug)]
pub struct HttpError {
    pub status: Option<u16>,
    pub message: String,
}

/// Generic HTTP client trait - users can implement their own
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// POST JSON data to URL, return response bytes
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError>;
}

#[cfg(feature = "reqwest")]
pub mod reqwest;

#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestClient;
