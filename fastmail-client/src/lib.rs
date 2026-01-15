pub mod client;
pub mod config;
pub mod whitelist;

pub use client::FastmailClient;
pub use config::Config;
pub use whitelist::Whitelist;

// Re-export JMAP types for convenience
pub use jmap_client::{MaskedEmail, MaskedEmailState};
