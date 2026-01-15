// jmap-client/src/lib.rs
pub mod client;
pub mod http;
pub mod types;

pub use client::JmapClient;
pub use http::{HttpClient, HttpError};
pub use types::{Email, EmailAddress, MaskedEmailState, Session};

// Re-export reqwest client when feature is enabled
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;
