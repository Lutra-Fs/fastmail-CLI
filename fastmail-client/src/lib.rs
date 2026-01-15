pub mod client;
pub mod config;
pub mod masked_email;
pub mod whitelist;

pub use client::FastmailClient;
pub use config::Config;
pub use masked_email::{MaskedEmail, MaskedEmailState};
pub use whitelist::Whitelist;
