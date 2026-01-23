// jmap-client/src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// JMAP Email object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    #[serde(rename = "blobId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,
    #[serde(rename = "threadId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(rename = "mailboxIds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mailbox_ids: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<HashMap<String, bool>>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(rename = "receivedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
    // Header fields
    #[serde(rename = "messageId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Vec<String>>,
    #[serde(rename = "inReplyTo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Vec<EmailAddress>>,
    #[serde(default)]
    pub from: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<EmailAddress>>,
    #[serde(rename = "replyTo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Vec<EmailAddress>>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(rename = "sentAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    // Body
    #[serde(rename = "bodyStructure")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_structure: Option<EmailBodyPart>,
    #[serde(rename = "bodyValues")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_values: Option<serde_json::Value>,
    #[serde(rename = "textBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<Vec<BodyPart>>,
    #[serde(rename = "htmlBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<Vec<BodyPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<BodyPart>>,
    #[serde(rename = "hasAttachment")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// JMAP Thread object (RFC 8621 §3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// The id of the Thread
    pub id: String,
    /// The ids of the Emails in this Thread, sorted by receivedAt date
    #[serde(rename = "emailIds")]
    pub email_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Email body part structure (RFC 8621 §4.1.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBodyPart {
    #[serde(rename = "partId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
    #[serde(rename = "blobId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<EmailHeader>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disposition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(rename = "subParts")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_parts: Option<Vec<EmailBodyPart>>,
}

/// Email header (RFC 8621 §4.1.3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailHeader {
    pub name: String,
    pub value: String,
}

/// Email query filter (RFC 8621 §4.4.1)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmailFilterCondition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_mailbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_mailbox_other_than: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_in_thread_have_keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub some_in_thread_have_keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_in_thread_have_keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachment: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<String>>,
}

/// SearchSnippet object (RFC 8621 §5)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSnippet {
    /// The Email id this snippet refers to
    #[serde(rename = "emailId")]
    pub email_id: String,
    /// Snippet from the subject (null if no match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Snippet from the email body (null if no match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Identity object (RFC 8621 §6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "replyTo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<EmailAddress>>,
    #[serde(rename = "textSignature")]
    #[serde(default)]
    pub text_signature: String,
    #[serde(rename = "htmlSignature")]
    #[serde(default)]
    pub html_signature: String,
    #[serde(rename = "mayDelete")]
    #[serde(default)]
    pub may_delete: bool,
}

/// EmailSubmission object (RFC 8621 §7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSubmission {
    pub id: String,
    #[serde(rename = "identityId")]
    pub identity_id: String,
    #[serde(rename = "emailId")]
    pub email_id: String,
    #[serde(rename = "threadId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<Envelope>,
    #[serde(rename = "sendAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_at: Option<String>,
    #[serde(rename = "undoStatus")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_status: Option<UndoStatus>,
    #[serde(rename = "deliveryStatus")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_status: Option<HashMap<String, DeliveryStatus>>,
    #[serde(rename = "dsnBlobIds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsn_blob_ids: Option<Vec<String>>,
    #[serde(rename = "mdnBlobIds")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mdn_blob_ids: Option<Vec<String>>,
}

/// SMTP envelope (RFC 8621 §7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    #[serde(rename = "mailFrom")]
    pub mail_from: EmailAddress,
    #[serde(rename = "rcptTo")]
    pub rcpt_to: Vec<EmailAddress>,
}

/// Undo status for delayed send (RFC 8621 §7)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UndoStatus {
    Pending,
    Final,
    Canceled,
}

/// Per-recipient delivery status (RFC 8621 §7)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStatus {
    #[serde(rename = "smtpReply")]
    pub smtp_reply: String,
    pub delivered: String,
    pub displayed: String,
}

/// VacationResponse object (RFC 8621 §8)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VacationResponse {
    pub id: String,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "fromDate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(rename = "toDate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "textBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
    #[serde(rename = "htmlBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
}

/// Email creation object (RFC 8621 §4.6)
#[derive(Debug, Clone, Serialize, Default)]
pub struct EmailCreate {
    #[serde(rename = "mailboxIds")]
    pub mailbox_ids: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<HashMap<String, bool>>,
    #[serde(rename = "receivedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
    // Header fields
    #[serde(rename = "messageId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Vec<String>>,
    #[serde(rename = "inReplyTo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<EmailAddress>>,
    #[serde(rename = "replyTo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Vec<EmailAddress>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(rename = "sentAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    // Body
    #[serde(rename = "bodyStructure")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_structure: Option<EmailBodyPart>,
    #[serde(rename = "bodyValues")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_values: Option<HashMap<String, EmailBodyValue>>,
    #[serde(rename = "textBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<Vec<BodyPart>>,
    #[serde(rename = "htmlBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<Vec<BodyPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<BodyPart>>,
}

/// Body value for email creation (RFC 8621 §4.1.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBodyValue {
    pub value: String,
    #[serde(rename = "isEncodingProblem")]
    #[serde(default)]
    pub is_encoding_problem: bool,
    #[serde(rename = "isTruncated")]
    #[serde(default)]
    pub is_truncated: bool,
}

/// Email import object (RFC 8621 §4.8)
#[derive(Debug, Clone, Serialize)]
pub struct EmailImport {
    #[serde(rename = "blobId")]
    pub blob_id: String,
    #[serde(rename = "mailboxIds")]
    pub mailbox_ids: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<HashMap<String, bool>>,
    #[serde(rename = "receivedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
}

/// JMAP Mailbox object (RFC 8621 §2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub name: String,
    #[serde(rename = "parentId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(rename = "sortOrder")]
    #[serde(default)]
    pub sort_order: u32,
    #[serde(rename = "totalEmails")]
    #[serde(default)]
    pub total_emails: u64,
    #[serde(rename = "unreadEmails")]
    #[serde(default)]
    pub unread_emails: u64,
    #[serde(rename = "totalThreads")]
    #[serde(default)]
    pub total_threads: u64,
    #[serde(rename = "unreadThreads")]
    #[serde(default)]
    pub unread_threads: u64,
    #[serde(rename = "myRights")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub my_rights: Option<MailboxRights>,
    #[serde(rename = "isSubscribed")]
    #[serde(default)]
    pub is_subscribed: bool,
}

/// Mailbox access rights (RFC 8621 §2)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MailboxRights {
    #[serde(rename = "mayReadItems")]
    pub may_read_items: bool,
    #[serde(rename = "mayAddItems")]
    pub may_add_items: bool,
    #[serde(rename = "mayRemoveItems")]
    pub may_remove_items: bool,
    #[serde(rename = "maySetSeen")]
    pub may_set_seen: bool,
    #[serde(rename = "maySetKeywords")]
    pub may_set_keywords: bool,
    #[serde(rename = "mayCreateChild")]
    pub may_create_child: bool,
    #[serde(rename = "mayRename")]
    pub may_rename: bool,
    #[serde(rename = "mayDelete")]
    pub may_delete: bool,
    #[serde(rename = "maySubmit")]
    pub may_submit: bool,
}

/// Mailbox query filter (RFC 8621 §2.3)
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MailboxFilterCondition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_any_role: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_subscribed: Option<bool>,
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
    /// Server capabilities (required)
    pub capabilities: HashMap<String, serde_json::Value>,
    /// The URL to use for JMAP API requests
    #[serde(rename = "apiUrl")]
    pub api_url: String,
    /// Download URL template for binary data
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    /// Upload URL template for files
    #[serde(rename = "uploadUrl")]
    pub upload_url: Option<String>,
    /// Event source URL for push notifications
    #[serde(rename = "eventSourceUrl")]
    pub event_source_url: Option<String>,
    /// The accounts available to the user
    pub accounts: HashMap<String, AccountData>,
    /// Map of capability URI to account ID for primary accounts
    #[serde(rename = "primaryAccounts")]
    #[serde(default)]
    pub primary_accounts: HashMap<String, String>,
    /// Username associated with credentials
    #[serde(default)]
    pub username: Option<String>,
    /// A string representing the state of this object on the server
    #[serde(default)]
    pub state: Option<String>,
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

/// JMAP Core capability (urn:ietf:params:jmap:core)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoreCapability {
    #[serde(rename = "maxSizeUpload")]
    pub max_size_upload: u64,
    #[serde(rename = "maxConcurrentUpload")]
    pub max_concurrent_upload: u64,
    #[serde(rename = "maxSizeRequest")]
    pub max_size_request: u64,
    #[serde(rename = "maxConcurrentRequests")]
    pub max_concurrent_requests: u64,
    #[serde(rename = "maxCallsInRequest")]
    pub max_calls_in_request: u64,
    #[serde(rename = "maxObjectsInGet")]
    pub max_objects_in_get: u64,
    #[serde(rename = "maxObjectsInSet")]
    pub max_objects_in_set: u64,
    #[serde(rename = "collationAlgorithms")]
    pub collation_algorithms: Vec<String>,
}

// Filter and Comparator types (RFC 8620 §5.5)

/// Filter operator for combining conditions (RFC 8620 §5.5)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FilterOperator {
    And,
    Or,
    Not,
}

/// Generic filter condition wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter<T> {
    /// Compound filter with operator
    Compound {
        operator: FilterOperator,
        conditions: Vec<Filter<T>>,
    },
    /// Simple condition
    Condition(T),
}

/// Sort comparator (RFC 8620 §5.5)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparator {
    pub property: String,
    #[serde(rename = "isAscending")]
    #[serde(default = "default_true")]
    pub is_ascending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Comparator {
    pub fn new(property: &str) -> Self {
        Self {
            property: property.to_string(),
            is_ascending: true,
            collation: None,
        }
    }

    pub fn desc(property: &str) -> Self {
        Self {
            property: property.to_string(),
            is_ascending: false,
            collation: None,
        }
    }
}

// Set and Error types (RFC 8620 §5.3, §3.6)

/// Generic /set response (RFC 8620 §5.3)
#[derive(Debug, Clone, Deserialize)]
pub struct SetResponse<T> {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "oldState")]
    pub old_state: Option<String>,
    #[serde(rename = "newState")]
    pub new_state: String,
    #[serde(default)]
    pub created: HashMap<String, T>,
    #[serde(default)]
    pub updated: HashMap<String, Option<T>>,
    #[serde(default)]
    pub destroyed: Vec<String>,
    #[serde(rename = "notCreated")]
    #[serde(default)]
    pub not_created: HashMap<String, SetError>,
    #[serde(rename = "notUpdated")]
    #[serde(default)]
    pub not_updated: HashMap<String, SetError>,
    #[serde(rename = "notDestroyed")]
    #[serde(default)]
    pub not_destroyed: HashMap<String, SetError>,
}

/// Error in /set method (RFC 8620 §5.3)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetError {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<String>>,
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

// Push types (RFC 8620 §7.2)

/// PushSubscription object (RFC 8620 §7.2)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PushSubscription {
    pub id: String,
    #[serde(rename = "deviceClientId")]
    pub device_client_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<PushKeys>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<String>>,
}

/// Keys for encrypted push (RFC 8620 §7.2)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
}

/// Changes response (RFC 8620 §5.2)
#[derive(Debug, Clone, Deserialize)]
pub struct ChangesResponse {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "oldState")]
    pub old_state: String,
    #[serde(rename = "newState")]
    pub new_state: String,
    #[serde(rename = "hasMoreChanges")]
    pub has_more_changes: bool,
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub destroyed: Vec<String>,
}

/// QueryChanges response (RFC 8620 §5.6)
#[derive(Debug, Clone, Deserialize)]
pub struct QueryChangesResponse {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "oldQueryState")]
    pub old_query_state: String,
    #[serde(rename = "newQueryState")]
    pub new_query_state: String,
    pub added: Vec<AddedItem>,
    pub removed: Vec<String>,
}

/// Added item in QueryChanges (RFC 8620 §5.6)
#[derive(Debug, Clone, Deserialize)]
pub struct AddedItem {
    pub id: String,
    pub index: usize,
}

/// Blob/copy response (RFC 8620 §6.3)
#[derive(Debug, Clone, Deserialize)]
pub struct BlobCopyResponse {
    #[serde(rename = "fromAccountId")]
    pub from_account_id: String,
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub copied: HashMap<String, String>,
    #[serde(rename = "notCopied")]
    #[serde(default)]
    pub not_copied: HashMap<String, serde_json::Value>,
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

    #[test]
    fn test_mailbox_filter_serialization() {
        let filter = MailboxFilterCondition {
            parent_id: None,
            name: None,
            role: Some("inbox".to_string()),
            has_any_role: None,
            is_subscribed: None,
        };
        let json = serde_json::to_value(filter).unwrap();
        assert_eq!(json, serde_json::json!({"role": "inbox"}));
    }

    #[test]
    fn test_email_filter_serialization() {
        let filter = EmailFilterCondition {
            in_mailbox: Some("inbox-id".to_string()),
            in_mailbox_other_than: None,
            before: None,
            after: None,
            min_size: None,
            max_size: None,
            all_in_thread_have_keyword: None,
            some_in_thread_have_keyword: None,
            none_in_thread_have_keyword: None,
            has_keyword: None,
            not_keyword: None,
            has_attachment: Some(true),
            text: None,
            from: None,
            to: None,
            cc: None,
            bcc: None,
            subject: None,
            body: None,
            header: None,
        };
        let json = serde_json::to_value(filter).unwrap();
        assert_eq!(json["inMailbox"], "inbox-id");
        assert_eq!(json["hasAttachment"], true);
    }

    #[test]
    fn test_comparator_creation() {
        let asc = Comparator::new("receivedAt");
        assert!(asc.is_ascending);

        let desc = Comparator::desc("size");
        assert!(!desc.is_ascending);
    }

    #[test]
    fn test_identity_deserialization() {
        let json = serde_json::json!({
            "id": "id1",
            "name": "Test User",
            "email": "test@example.com",
            "textSignature": "-- Sent from JMAP",
            "htmlSignature": "",
            "mayDelete": true
        });
        let identity: Identity = serde_json::from_value(json).unwrap();
        assert_eq!(identity.name, "Test User");
        assert!(identity.may_delete);
    }

    #[test]
    fn test_vacation_response_serialization() {
        let vacation = VacationResponse {
            id: "singleton".to_string(),
            is_enabled: true,
            from_date: Some("2026-01-01T00:00:00Z".to_string()),
            to_date: Some("2026-01-15T00:00:00Z".to_string()),
            subject: Some("Out of Office".to_string()),
            text_body: Some("I am away".to_string()),
            html_body: None,
        };
        let json = serde_json::to_value(vacation).unwrap();
        assert_eq!(json["isEnabled"], true);
    }
}
