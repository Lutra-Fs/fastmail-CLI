// jmap-client/src/types.rs
use serde::{Deserialize, Serialize};

/// JMAP Email object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub from: EmailAddress,
    pub subject: String,
    #[serde(rename = "receivedAt")]
    pub received_at: String,
    #[serde(default)]
    pub preview: String,
    #[serde(rename = "bodyValues")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_values: Option<serde_json::Value>,
    #[serde(rename = "textBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<Vec<BodyPart>>,
    #[serde(rename = "htmlBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<Vec<BodyPart>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPart {
    #[serde(rename = "partId")]
    pub part_id: String,
    #[serde(rename = "blobId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(rename = "type")]
    pub type_: String,
}

/// JMAP Session response
#[derive(Debug, Deserialize)]
pub struct Session {
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    pub accounts: Vec<AccountData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    #[serde(rename = "id")]
    pub account_id: String,
}

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
