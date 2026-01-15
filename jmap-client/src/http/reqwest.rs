// jmap-client/src/http/reqwest.rs
use super::{HttpClient, HttpError};
use async_trait::async_trait;

#[cfg(feature = "reqwest")]
pub struct ReqwestClient {
    inner: reqwest::Client,
    bearer_token: Option<String>,
}

#[cfg(feature = "reqwest")]
impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
            bearer_token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.bearer_token = Some(token);
        self
    }
}

#[cfg(feature = "reqwest")]
impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "reqwest")]
#[async_trait]
impl HttpClient for ReqwestClient {
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        let mut req = self.inner.post(url);

        if let Some(token) = &self.bearer_token {
            req = req.bearer_auth(token);
        }

        let resp = req
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| HttpError {
                status: None,
                message: e.to_string(),
            })?;

        let status = resp.status();
        let is_success = status.is_success();
        let status_code = status.as_u16();

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| HttpError {
                status: Some(status_code),
                message: e.to_string(),
            })?
            .to_vec();

        if !is_success {
            return Err(HttpError {
                status: Some(status_code),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        Ok(bytes)
    }
}
