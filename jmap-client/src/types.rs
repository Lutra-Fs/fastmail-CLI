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

/// JMAP Session response (RFC 8620 Section 2)
#[derive(Debug, Deserialize)]
pub struct Session {
    /// The URL to use for JMAP API requests
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    /// Download URL template for binary data
    #[serde(default)]
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    /// Upload URL template for files
    #[serde(default)]
    #[serde(rename = "uploadUrl")]
    pub upload_url: Option<String>,
    /// Event source URL for push notifications
    #[serde(default)]
    #[serde(rename = "eventSourceUrl")]
    pub event_source_url: Option<String>,
    /// The accounts available to the user
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

// Blob types (RFC 9404)

/// JMAP Blob capability (urn:ietf:params:jmap:blob)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlobCapability {
    #[serde(rename = "maxSizeBlobSet")]
    pub max_size_blob_set: Option<u64>,
    #[serde(rename = "maxDataSources")]
    pub max_data_sources: u64,
    #[serde(rename = "supportedTypeNames")]
    pub supported_type_names: Vec<String>,
    #[serde(rename = "supportedDigestAlgorithms")]
    pub supported_digest_algorithms: Vec<String>,
}

/// Blob/upload request object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobUploadObject {
    pub data: Vec<DataSourceObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

/// Data source for blob upload - one of text, base64, or blob reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataSourceObject {
    AsText {
        #[serde(rename = "data:asText")]
        data_as_text: String,
    },
    AsBase64 {
        #[serde(rename = "data:asBase64")]
        data_as_base64: String,
    },
    BlobRef {
        #[serde(rename = "blobId")]
        blob_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        length: Option<u64>,
    },
}

/// Response when a blob is created (RFC 9404)
#[derive(Debug, Clone, Deserialize)]
pub struct BlobCreatedInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub size: u64,
}

/// Response from blob upload (RFC 9404)
#[derive(Debug, Clone, Deserialize)]
pub struct BlobUploadResponse {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(default)]
    pub created: std::collections::HashMap<String, BlobCreatedInfo>,
    #[serde(default)]
    #[serde(rename = "notCreated")]
    pub not_created: std::collections::HashMap<String, serde_json::Value>,
}

/// Response from blob get (RFC 9404)
#[derive(Debug, Clone, Deserialize)]
pub struct BlobGetResponse {
    pub id: String,
    #[serde(rename = "data:asText")]
    #[serde(default)]
    pub data_as_text: Option<String>,
    #[serde(rename = "data:asBase64")]
    #[serde(default)]
    pub data_as_base64: Option<String>,
    /// Dynamic digest properties (digest:sha, digest:sha-256, etc.)
    #[serde(flatten)]
    pub digests: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub size: u64,
    #[serde(rename = "isEncodingProblem")]
    #[serde(default)]
    pub is_encoding_problem: bool,
    #[serde(rename = "isTruncated")]
    #[serde(default)]
    pub is_truncated: bool,
}

impl BlobGetResponse {
    /// Get digest value for algorithm if present
    pub fn digest(&self, algorithm: &str) -> Option<&String> {
        self.digests.get(&format!("digest:{}", algorithm))
    }

    /// Get data as bytes (decodes base64 if needed)
    pub fn as_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        if let Some(text) = &self.data_as_text {
            Ok(text.as_bytes().to_vec())
        } else if let Some(b64) = &self.data_as_base64 {
            crate::blob::decode_base64(b64)
        } else {
            Err(anyhow::anyhow!("No data available"))
        }
    }

    /// Get data as text if UTF-8 valid
    pub fn as_text(&self) -> Result<String, anyhow::Error> {
        self.data_as_text
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Data not valid UTF-8"))
    }
}

/// Info from blob lookup (RFC 9404)
#[derive(Debug, Clone, Deserialize)]
pub struct BlobLookupInfo {
    pub id: String,
    #[serde(rename = "matchedIds")]
    pub matched_ids: std::collections::HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_data_source_text_serialization() {
        let ds = DataSourceObject::AsText {
            data_as_text: "hello".to_string(),
        };
        let json = serde_json::to_value(ds).unwrap();
        assert_eq!(json, json!({"data:asText": "hello"}));
    }

    #[test]
    fn test_data_source_base64_serialization() {
        let ds = DataSourceObject::AsBase64 {
            data_as_base64: "SGVsbG8=".to_string(),
        };
        let json = serde_json::to_value(ds).unwrap();
        assert_eq!(json, json!({"data:asBase64": "SGVsbG8="}));
    }

    #[test]
    fn test_data_source_blob_ref_serialization() {
        let ds = DataSourceObject::BlobRef {
            blob_id: "G123".to_string(),
            offset: Some(10),
            length: Some(100),
        };
        let json = serde_json::to_value(ds).unwrap();
        assert_eq!(
            json,
            json!({"blobId": "G123", "offset": 10, "length": 100})
        );
    }

    #[test]
    fn test_blob_get_response_digest() {
        let json = json!({
            "id": "G123",
            "size": 100,
            "digest:sha": "abc123",
            "digest:sha-256": "def456"
        });
        let resp: BlobGetResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.digest("sha"), Some(&"abc123".to_string()));
        assert_eq!(resp.digest("sha-256"), Some(&"def456".to_string()));
        assert_eq!(resp.digest("md5"), None);
    }
}
