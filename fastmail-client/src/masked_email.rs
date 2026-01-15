use serde::{Deserialize, Serialize};

/// Masked Email (Fastmail extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedEmail {
    pub id: String,
    pub email: String,
    pub state: MaskedEmailState,
    #[serde(rename = "forDomain")]
    pub for_domain: String,
    pub description: String,
    #[serde(rename = "lastMessageAt")]
    pub last_message_at: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub url: Option<String>,
}

/// Masked Email state (Fastmail extension)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MaskedEmailState {
    Pending,
    Enabled,
    Disabled,
    Deleted,
}
