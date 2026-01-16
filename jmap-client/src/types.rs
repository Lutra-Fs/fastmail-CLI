// jmap-client/src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

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
    pub created: HashMap<String, BlobCreatedInfo>,
    #[serde(default)]
    #[serde(rename = "notCreated")]
    pub not_created: HashMap<String, serde_json::Value>,
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
    pub digests: HashMap<String, String>,
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
    pub matched_ids: HashMap<String, Vec<String>>,
}

// Principal types (RFC 9670)

/// JMAP Principals capability (urn:ietf:params:jmap:principals)
#[derive(Debug, Clone, Deserialize)]
pub struct PrincipalsCapability {
    /// Empty object for session-level capability
    #[serde(default)]
    pub _empty: serde_json::Value,
}

/// Account-level principals capability (urn:ietf:params:jmap:principals)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrincipalsAccountCapability {
    /// The id of the Principal that corresponds to the user fetching this object
    #[serde(rename = "currentUserPrincipalId")]
    pub current_user_principal_id: Option<String>,
}

/// Owner capability (urn:ietf:params:jmap:principals:owner)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PrincipalsOwnerCapability {
    /// The id of an Account with the urn:ietf:params:jmap:principals capability
    /// that contains the corresponding Principal object
    #[serde(rename = "accountIdForPrincipal")]
    pub account_id_for_principal: String,
    /// The id of the Principal that owns this Account
    #[serde(rename = "principalId")]
    pub principal_id: String,
}

/// Principal type (RFC 9670 Section 5)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrincipalType {
    /// A single person
    Individual,
    /// A group of other Principals
    Group,
    /// A resource (e.g., a projector)
    Resource,
    /// A location (e.g., a room)
    Location,
    /// Some other undefined Principal
    Other,
}

/// JMAP Principal object (RFC 9670)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Principal {
    /// The id of the Principal
    pub id: String,
    /// The type of Principal
    #[serde(rename = "type")]
    pub type_: PrincipalType,
    /// The name of the Principal
    pub name: String,
    /// A longer description
    pub description: Option<String>,
    /// An email address for the Principal
    pub email: Option<String>,
    /// The time zone for this Principal
    pub time_zone: Option<String>,
    /// Domain-specific capabilities
    pub capabilities: HashMap<String, serde_json::Value>,
    /// Map of Account id to Account object for each JMAP Account
    /// containing data for this Principal that the user has access to
    pub accounts: Option<HashMap<String, AccountData>>,
}

/// Principal query filter condition (RFC 9670 Section 5.5)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrincipalFilterCondition {
    /// List of Account ids - Principal matches if any are keys in Principal's accounts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_ids: Option<Vec<String>>,
    /// Email property contains the given string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Name property contains the given string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Name, email, or description contains the given string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Type must match exactly
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<PrincipalType>,
    /// TimeZone must match exactly
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

/// Principal/query sort option (RFC 9670)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrincipalSortProperty {
    Name,
    Email,
    Type,
}

// ShareNotification types (RFC 9670)

/// Entity that made a change (RFC 9670 Section 6)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// The name of the entity who made the change
    pub name: String,
    /// The email of the entity who made the change
    pub email: Option<String>,
    /// The id of the Principal corresponding to the entity
    #[serde(rename = "principalId")]
    pub principal_id: Option<String>,
}

/// ShareNotification object (RFC 9670 Section 6)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareNotification {
    /// The id of the ShareNotification
    pub id: String,
    /// The time this notification was created
    pub created: DateTime<Utc>,
    /// Who made the change
    #[serde(rename = "changedBy")]
    pub changed_by: Entity,
    /// The name of the data type for the object whose permissions have changed
    #[serde(rename = "objectType")]
    pub object_type: String,
    /// The id of the Account where this object exists
    #[serde(rename = "objectAccountId")]
    pub object_account_id: String,
    /// The id of the object that this notification is about
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// The myRights property before the change
    #[serde(rename = "oldRights")]
    pub old_rights: Option<HashMap<String, bool>>,
    /// The myRights property after the change
    #[serde(rename = "newRights")]
    pub new_rights: Option<HashMap<String, bool>>,
    /// The name of the object at the time of notification
    pub name: String,
}

/// ShareNotification query filter condition (RFC 9670 Section 6.5)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShareNotificationFilterCondition {
    /// Creation date must be on or after this date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<DateTime<Utc>>,
    /// Creation date must be before this date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<DateTime<Utc>>,
    /// ObjectType must match exactly
    #[serde(rename = "objectType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// ObjectAccountId must match exactly
    #[serde(rename = "objectAccountId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_account_id: Option<String>,
}

/// ShareNotification sort property
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShareNotificationSortProperty {
    Created,
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

    #[test]
    fn test_principal_type_serialization() {
        use serde_json::json;

        let pt = PrincipalType::Individual;
        let json = serde_json::to_value(pt).unwrap();
        assert_eq!(json, json!("individual"));

        let pt = PrincipalType::Group;
        let json = serde_json::to_value(pt).unwrap();
        assert_eq!(json, json!("group"));

        let pt = PrincipalType::Resource;
        let json = serde_json::to_value(pt).unwrap();
        assert_eq!(json, json!("resource"));

        let pt = PrincipalType::Location;
        let json = serde_json::to_value(pt).unwrap();
        assert_eq!(json, json!("location"));

        let pt = PrincipalType::Other;
        let json = serde_json::to_value(pt).unwrap();
        assert_eq!(json, json!("other"));
    }

    #[test]
    fn test_principal_filter_condition_serialization() {
        use serde_json::json;

        let filter = PrincipalFilterCondition {
            account_ids: None,
            email: Some("test@example.com".to_string()),
            name: None,
            text: None,
            type_: Some(PrincipalType::Individual),
            time_zone: None,
        };

        let json = serde_json::to_value(filter).unwrap();
        assert_eq!(
            json,
            json!({
                "email": "test@example.com",
                "type": "individual"
            })
        );
    }
}
