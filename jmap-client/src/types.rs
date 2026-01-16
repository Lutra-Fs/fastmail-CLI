// jmap-client/src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JMAP Email object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    #[serde(default)]
    pub from: Option<Vec<EmailAddress>>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(rename = "receivedAt")]
    #[serde(default)]
    pub received_at: Option<String>,
    #[serde(default)]
    pub preview: Option<String>,
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

/// JMAP Mailbox object (minimal fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub name: String,
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
    pub accounts: HashMap<String, AccountData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub name: Option<String>,
    #[serde(rename = "isPersonal")]
    pub is_personal: Option<bool>,
    #[serde(rename = "isReadOnly")]
    pub is_read_only: Option<bool>,
    #[serde(rename = "accountCapabilities")]
    pub account_capabilities: Option<HashMap<String, serde_json::Value>>,
}

// Blob types - placeholders to be implemented in later tasks
// These will be properly implemented in upcoming tasks

/// Blob capability from server (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobCapability {
    #[serde(rename = "maxSize")]
    pub max_size: Option<u64>,
}

/// Blob upload object (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobUploadObject {
    #[serde(rename = "data")]
    pub data: DataSourceObject,
}

/// Data source object for blob uploads (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceObject {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: String,
}

/// Response when a blob is created (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobCreatedInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: u64,
}

/// Response from blob upload (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobUploadResponse {
    #[serde(rename = "blobId")]
    pub blob_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: u64,
}

/// Response from blob get (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobGetResponse {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "blobId")]
    pub blob_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: u64,
    pub expires: String,
}

/// Info from blob lookup (RFC 9404)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobLookupInfo {
    #[serde(rename = "blobId")]
    pub blob_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub size: u64,
}
