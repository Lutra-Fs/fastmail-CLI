# JMAP RFC 8620 & RFC 8621 Full Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete RFC 8620 (JMAP Core) and RFC 8621 (JMAP Mail) implementation in `jmap-client` crate.

**Current Status:** Partial implementation exists. Need to add missing data types, methods, and complete existing types.

---

## Gap Analysis

### RFC 8620 (JMAP Core) - Current vs Required

| Component | Status | Notes |
|-----------|--------|-------|
| Session object (§2) | ✅ Partial | Missing `capabilities`, `state`, `primaryAccounts` |
| Core/echo (§4) | ✅ Done | |
| /get pattern (§5.1) | ✅ Done | Generic implementation |
| /changes pattern (§5.2) | ✅ Done | `ChangesResponse` type exists |
| /set pattern (§5.3) | ✅ Partial | Missing `SetResponse` type, `ifInState` |
| /copy pattern (§5.4) | ❌ Missing | Need generic copy support |
| /query pattern (§5.5) | ✅ Partial | Missing `Filter`, `Comparator` types |
| /queryChanges pattern (§5.6) | ✅ Done | `QueryChangesResponse` exists |
| Binary upload (§6.1) | ✅ Done | Via `uploadUrl` |
| Binary download (§6.2) | ✅ Done | Via `downloadUrl` |
| Blob/copy (§6.3) | ✅ Done | `BlobCopyResponse` exists |
| PushSubscription (§7.2) | ✅ Done | get/set methods exist |
| EventSource (§7.3) | ❌ Missing | SSE connection support |
| Result references (§3.7) | ❌ Missing | `#` reference syntax |
| Error types (§3.6) | ❌ Missing | Typed error responses |

### RFC 8621 (JMAP Mail) - Current vs Required

| Component | Status | Notes |
|-----------|--------|-------|
| **Mailbox** (§2) | | |
| - Mailbox object | ✅ Partial | Missing many properties |
| - Mailbox/get | ✅ Done | |
| - Mailbox/changes | ✅ Done | |
| - Mailbox/query | ❌ Missing | Filter/sort for mailboxes |
| - Mailbox/queryChanges | ❌ Missing | |
| - Mailbox/set | ✅ Partial | Create/destroy only, no update |
| **Thread** (§3) | | |
| - Thread object | ❌ Missing | |
| - Thread/get | ❌ Missing | |
| - Thread/changes | ❌ Missing | |
| **Email** (§4) | | |
| - Email object | ✅ Partial | Missing many properties |
| - Email/get | ✅ Done | |
| - Email/changes | ✅ Done | |
| - Email/query | ✅ Partial | Basic filter only |
| - Email/queryChanges | ✅ Done | |
| - Email/set | ✅ Partial | Delete only, no create/update |
| - Email/copy | ❌ Missing | |
| - Email/import | ❌ Missing | Import RFC 5322 message |
| - Email/parse | ❌ Missing | Parse blob as email |
| **SearchSnippet** (§5) | | |
| - SearchSnippet object | ❌ Missing | |
| - SearchSnippet/get | ❌ Missing | |
| **Identity** (§6) | | |
| - Identity object | ❌ Missing | |
| - Identity/get | ❌ Missing | |
| - Identity/changes | ❌ Missing | |
| - Identity/set | ❌ Missing | |
| **EmailSubmission** (§7) | | |
| - EmailSubmission object | ❌ Missing | |
| - EmailSubmission/get | ❌ Missing | |
| - EmailSubmission/changes | ❌ Missing | |
| - EmailSubmission/query | ❌ Missing | |
| - EmailSubmission/queryChanges | ❌ Missing | |
| - EmailSubmission/set | ❌ Missing | |
| **VacationResponse** (§8) | | |
| - VacationResponse object | ❌ Missing | |
| - VacationResponse/get | ❌ Missing | |
| - VacationResponse/set | ❌ Missing | |

---

## Implementation Tasks

### Phase 1: RFC 8620 Core Completion

#### Task 1: Complete Session Type

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add missing Session fields**

```rust
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
```

**Step 2: Add CoreCapability type**

```rust
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
```

**Step 3: Build and test**

Run: `cargo build -p jmap-client`
Expected: SUCCESS

---

#### Task 2: Add Generic Filter and Comparator Types

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add FilterOperator enum**

```rust
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
```

**Step 2: Add Comparator type**

```rust
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

fn default_true() -> bool { true }

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
```

**Step 3: Build**

Run: `cargo build -p jmap-client`

---

#### Task 3: Add SetResponse and Error Types

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/error.rs`

**Step 1: Add SetResponse type**

```rust
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
```

**Step 2: Add SetError type**

```rust
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
```

**Step 3: Add MethodError type in error.rs**

```rust
/// JMAP method-level error (RFC 8620 §3.6.2)
#[derive(Debug, Clone, Deserialize)]
pub struct MethodError {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Common JMAP error types
pub mod error_types {
    pub const UNKNOWN_CAPABILITY: &str = "urn:ietf:params:jmap:error:unknownCapability";
    pub const NOT_JSON: &str = "urn:ietf:params:jmap:error:notJSON";
    pub const NOT_REQUEST: &str = "urn:ietf:params:jmap:error:notRequest";
    pub const LIMIT: &str = "urn:ietf:params:jmap:error:limit";
    
    // Method-level
    pub const SERVER_UNAVAILABLE: &str = "serverUnavailable";
    pub const SERVER_FAIL: &str = "serverFail";
    pub const SERVER_PARTIAL_FAIL: &str = "serverPartialFail";
    pub const UNKNOWN_METHOD: &str = "unknownMethod";
    pub const INVALID_ARGUMENTS: &str = "invalidArguments";
    pub const INVALID_RESULT_REFERENCE: &str = "invalidResultReference";
    pub const FORBIDDEN: &str = "forbidden";
    pub const ACCOUNT_NOT_FOUND: &str = "accountNotFound";
    pub const ACCOUNT_NOT_SUPPORTED_BY_METHOD: &str = "accountNotSupportedByMethod";
    pub const ACCOUNT_READ_ONLY: &str = "accountReadOnly";
}
```

**Step 4: Build**

Run: `cargo build -p jmap-client`

---

### Phase 2: RFC 8621 Data Types

#### Task 4: Complete Mailbox Type

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Expand Mailbox struct**

```rust
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
```

**Step 2: Add MailboxFilterCondition**

```rust
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
```

---

#### Task 5: Add Thread Type

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add Thread struct**

```rust
/// JMAP Thread object (RFC 8621 §3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// The id of the Thread
    pub id: String,
    /// The ids of the Emails in this Thread, sorted by receivedAt date
    #[serde(rename = "emailIds")]
    pub email_ids: Vec<String>,
}
```

**Step 2: Add Thread methods to client.rs**

```rust
/// Get Threads by IDs (RFC 8621 §3.1)
pub async fn thread_get(&self, ids: &[String]) -> Result<Vec<Thread>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let params = json!({
        "accountId": self.account_id,
        "ids": ids,
    });

    let args = self.call_method("Thread/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid Thread/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}

/// Get Thread changes since state (RFC 8621 §3.2)
pub async fn thread_changes(
    &self,
    since_state: &str,
    max_changes: Option<usize>,
) -> Result<ChangesResponse> {
    let mut params = json!({
        "accountId": self.account_id,
        "sinceState": since_state,
    });

    if let Some(mc) = max_changes {
        params["maxChanges"] = json!(mc);
    }

    let args = self.call_method("Thread/changes", params).await?;
    serde_json::from_value(args).map_err(Into::into)
}
```

---

#### Task 6: Complete Email Type

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Expand Email struct**

```rust
/// JMAP Email object (RFC 8621 §4)
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
```

**Step 2: Add EmailFilterCondition**

```rust
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
```

---

#### Task 7: Add SearchSnippet Type

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add SearchSnippet struct**

```rust
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
```

**Step 2: Add SearchSnippet/get method**

```rust
/// Get SearchSnippets for emails matching a filter (RFC 8621 §5.1)
pub async fn search_snippet_get(
    &self,
    email_ids: &[String],
    filter: Option<serde_json::Value>,
) -> Result<Vec<SearchSnippet>> {
    if email_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = json!({
        "accountId": self.account_id,
        "emailIds": email_ids,
    });

    if let Some(f) = filter {
        params["filter"] = f;
    }

    let args = self.call_method("SearchSnippet/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid SearchSnippet/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}
```

---

#### Task 8: Add Identity Type

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add Identity struct**

```rust
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
```

**Step 2: Add Identity methods**

```rust
const SUBMISSION_CAPABILITY: &str = "urn:ietf:params:jmap:submission";

/// Get all Identities (RFC 8621 §6.1)
pub async fn identity_get_all(&self) -> Result<Vec<Identity>> {
    let params = json!({
        "accountId": self.account_id,
        "ids": null,
    });

    let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
    let args = self.call_method_with_using(&using, "Identity/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid Identity/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}

/// Get Identity changes (RFC 8621 §6.2)
pub async fn identity_changes(
    &self,
    since_state: &str,
    max_changes: Option<usize>,
) -> Result<ChangesResponse> {
    let mut params = json!({
        "accountId": self.account_id,
        "sinceState": since_state,
    });

    if let Some(mc) = max_changes {
        params["maxChanges"] = json!(mc);
    }

    let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
    let args = self.call_method_with_using(&using, "Identity/changes", params).await?;
    serde_json::from_value(args).map_err(Into::into)
}
```

---

#### Task 9: Add EmailSubmission Type

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add EmailSubmission struct**

```rust
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
```

**Step 2: Add EmailSubmission methods**

```rust
/// Create and send an EmailSubmission (RFC 8621 §7.5)
pub async fn email_submission_create(
    &self,
    identity_id: &str,
    email_id: &str,
    envelope: Option<Envelope>,
) -> Result<EmailSubmission> {
    let mut create_obj = json!({
        "identityId": identity_id,
        "emailId": email_id,
    });

    if let Some(env) = envelope {
        create_obj["envelope"] = serde_json::to_value(env)?;
    }

    let params = json!({
        "accountId": self.account_id,
        "create": { "sub": create_obj },
    });

    let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
    let args = self.call_method_with_using(&using, "EmailSubmission/set", params).await?;

    // Check for errors
    if let Some(not_created) = args.get("notCreated") {
        if let Some(error) = not_created.get("sub") {
            anyhow::bail!("Failed to submit email: {}", error);
        }
    }

    let created = args
        .get("created")
        .and_then(|c| c.get("sub"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No created submission in response"))?;

    serde_json::from_value(created).map_err(Into::into)
}

/// Get EmailSubmissions by IDs (RFC 8621 §7.1)
pub async fn email_submission_get(&self, ids: &[String]) -> Result<Vec<EmailSubmission>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let params = json!({
        "accountId": self.account_id,
        "ids": ids,
    });

    let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
    let args = self.call_method_with_using(&using, "EmailSubmission/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid EmailSubmission/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}

/// Cancel a pending EmailSubmission (RFC 8621 §7.5)
pub async fn email_submission_cancel(&self, id: &str) -> Result<()> {
    let params = json!({
        "accountId": self.account_id,
        "update": { id: { "undoStatus": "canceled" } },
    });

    let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
    self.call_method_with_using(&using, "EmailSubmission/set", params).await?;
    Ok(())
}
```

---

#### Task 10: Add VacationResponse Type

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add VacationResponse struct**

```rust
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
```

**Step 2: Add VacationResponse methods**

```rust
const VACATION_CAPABILITY: &str = "urn:ietf:params:jmap:vacationresponse";

/// Get VacationResponse (RFC 8621 §8.1)
/// Note: There is only ever one VacationResponse per account with id "singleton"
pub async fn vacation_response_get(&self) -> Result<VacationResponse> {
    let params = json!({
        "accountId": self.account_id,
        "ids": ["singleton"],
    });

    let using = [CORE_CAPABILITY, VACATION_CAPABILITY];
    let args = self.call_method_with_using(&using, "VacationResponse/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid VacationResponse/get response: no list"))?;

    let first = list.first()
        .ok_or_else(|| anyhow::anyhow!("No VacationResponse in response"))?;

    serde_json::from_value(first.clone()).map_err(Into::into)
}

/// Update VacationResponse (RFC 8621 §8.2)
pub async fn vacation_response_set(
    &self,
    is_enabled: Option<bool>,
    from_date: Option<&str>,
    to_date: Option<&str>,
    subject: Option<&str>,
    text_body: Option<&str>,
    html_body: Option<&str>,
) -> Result<()> {
    let mut update = serde_json::Map::new();

    if let Some(v) = is_enabled {
        update.insert("isEnabled".to_string(), json!(v));
    }
    if let Some(v) = from_date {
        update.insert("fromDate".to_string(), json!(v));
    }
    if let Some(v) = to_date {
        update.insert("toDate".to_string(), json!(v));
    }
    if let Some(v) = subject {
        update.insert("subject".to_string(), json!(v));
    }
    if let Some(v) = text_body {
        update.insert("textBody".to_string(), json!(v));
    }
    if let Some(v) = html_body {
        update.insert("htmlBody".to_string(), json!(v));
    }

    let params = json!({
        "accountId": self.account_id,
        "update": { "singleton": update },
    });

    let using = [CORE_CAPABILITY, VACATION_CAPABILITY];
    self.call_method_with_using(&using, "VacationResponse/set", params).await?;
    Ok(())
}
```

---

### Phase 3: Email Creation and Import

#### Task 11: Add Email/set for Create/Update

**Files:**
- Modify: `jmap-client/src/client.rs`
- Modify: `jmap-client/src/types.rs`

**Step 1: Add EmailCreate type**

```rust
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
    pub text_body: Option<Vec<EmailBodyPart>>,
    #[serde(rename = "htmlBody")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<Vec<EmailBodyPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<EmailBodyPart>>,
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
```

**Step 2: Add email_create method**

```rust
/// Create a new Email (RFC 8621 §4.6)
pub async fn email_create(&self, email: EmailCreate) -> Result<Email> {
    let params = json!({
        "accountId": self.account_id,
        "create": { "new": email },
    });

    let args = self.call_method("Email/set", params).await?;

    // Check for errors
    if let Some(not_created) = args.get("notCreated") {
        if let Some(error) = not_created.get("new") {
            anyhow::bail!("Failed to create email: {}", error);
        }
    }

    let created = args
        .get("created")
        .and_then(|c| c.get("new"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No created email in response"))?;

    serde_json::from_value(created).map_err(Into::into)
}

/// Update an Email's mutable properties (RFC 8621 §4.6)
/// Only mailboxIds and keywords can be updated
pub async fn email_update(
    &self,
    id: &str,
    mailbox_ids: Option<HashMap<String, bool>>,
    keywords: Option<HashMap<String, bool>>,
) -> Result<()> {
    let mut update = serde_json::Map::new();

    if let Some(m) = mailbox_ids {
        update.insert("mailboxIds".to_string(), json!(m));
    }
    if let Some(k) = keywords {
        update.insert("keywords".to_string(), json!(k));
    }

    let params = json!({
        "accountId": self.account_id,
        "update": { id: update },
    });

    self.call_method("Email/set", params).await?;
    Ok(())
}
```

---

#### Task 12: Add Email/import and Email/copy

**Files:**
- Modify: `jmap-client/src/client.rs`
- Modify: `jmap-client/src/types.rs`

**Step 1: Add EmailImport type**

```rust
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
```

**Step 2: Add email_import method**

```rust
/// Import an RFC 5322 message from a blob (RFC 8621 §4.8)
pub async fn email_import(&self, import: EmailImport) -> Result<Email> {
    let params = json!({
        "accountId": self.account_id,
        "emails": { "import1": import },
    });

    let args = self.call_method("Email/import", params).await?;

    // Check for errors
    if let Some(not_created) = args.get("notCreated") {
        if let Some(error) = not_created.get("import1") {
            anyhow::bail!("Failed to import email: {}", error);
        }
    }

    let created = args
        .get("created")
        .and_then(|c| c.get("import1"))
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No imported email in response"))?;

    serde_json::from_value(created).map_err(Into::into)
}

/// Copy emails between accounts (RFC 8621 §4.7)
pub async fn email_copy(
    &self,
    from_account_id: &str,
    email_ids: &[String],
    mailbox_ids: HashMap<String, bool>,
) -> Result<HashMap<String, Email>> {
    let create: HashMap<String, serde_json::Value> = email_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                json!({
                    "mailboxIds": mailbox_ids.clone()
                }),
            )
        })
        .collect();

    let params = json!({
        "fromAccountId": from_account_id,
        "accountId": self.account_id,
        "create": create,
    });

    let args = self.call_method("Email/copy", params).await?;

    let created = args
        .get("created")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Invalid Email/copy response"))?;

    created
        .iter()
        .map(|(k, v)| {
            let email: Email = serde_json::from_value(v.clone())?;
            Ok((k.clone(), email))
        })
        .collect()
}
```

---

#### Task 13: Add Email/parse

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add email_parse method**

```rust
/// Parse a blob as an RFC 5322 message without storing it (RFC 8621 §4.9)
pub async fn email_parse(
    &self,
    blob_ids: &[String],
    properties: Option<Vec<String>>,
    body_properties: Option<Vec<String>>,
    fetch_text_body_values: Option<bool>,
    fetch_html_body_values: Option<bool>,
    fetch_all_body_values: Option<bool>,
    max_body_value_bytes: Option<u64>,
) -> Result<Vec<Email>> {
    let mut params = json!({
        "accountId": self.account_id,
        "blobIds": blob_ids,
    });

    if let Some(p) = properties {
        params["properties"] = json!(p);
    }
    if let Some(bp) = body_properties {
        params["bodyProperties"] = json!(bp);
    }
    if let Some(v) = fetch_text_body_values {
        params["fetchTextBodyValues"] = json!(v);
    }
    if let Some(v) = fetch_html_body_values {
        params["fetchHTMLBodyValues"] = json!(v);
    }
    if let Some(v) = fetch_all_body_values {
        params["fetchAllBodyValues"] = json!(v);
    }
    if let Some(v) = max_body_value_bytes {
        params["maxBodyValueBytes"] = json!(v);
    }

    let args = self.call_method("Email/parse", params).await?;

    let parsed = args
        .get("parsed")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Invalid Email/parse response"))?;

    parsed
        .values()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}
```

---

### Phase 4: Mailbox Query and Updates

#### Task 14: Add Mailbox/query and Mailbox/set Update

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add mailbox_query method**

```rust
/// Query Mailboxes with filter and sort (RFC 8621 §2.3)
pub async fn mailbox_query(
    &self,
    filter: Option<MailboxFilterCondition>,
    sort: Option<Vec<Comparator>>,
    limit: Option<usize>,
) -> Result<Vec<String>> {
    let mut params = json!({
        "accountId": self.account_id,
    });

    if let Some(f) = filter {
        params["filter"] = serde_json::to_value(f)?;
    }
    if let Some(s) = sort {
        params["sort"] = serde_json::to_value(s)?;
    }
    if let Some(l) = limit {
        params["limit"] = json!(l);
    }

    let args = self.call_method("Mailbox/query", params).await?;

    let ids_arr = args
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid Mailbox/query response: no ids"))?;

    Ok(ids_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect())
}

/// Update a Mailbox (RFC 8621 §2.5)
pub async fn mailbox_update(
    &self,
    id: &str,
    name: Option<&str>,
    parent_id: Option<Option<&str>>,
    is_subscribed: Option<bool>,
    sort_order: Option<u32>,
) -> Result<()> {
    let mut update = serde_json::Map::new();

    if let Some(n) = name {
        update.insert("name".to_string(), json!(n));
    }
    if let Some(p) = parent_id {
        update.insert("parentId".to_string(), json!(p));
    }
    if let Some(s) = is_subscribed {
        update.insert("isSubscribed".to_string(), json!(s));
    }
    if let Some(o) = sort_order {
        update.insert("sortOrder".to_string(), json!(o));
    }

    let params = json!({
        "accountId": self.account_id,
        "update": { id: update },
    });

    self.call_method("Mailbox/set", params).await?;
    Ok(())
}
```

---

### Phase 5: Update lib.rs Exports

#### Task 15: Export All New Types

**Files:**
- Modify: `jmap-client/src/lib.rs`

**Step 1: Update exports**

```rust
pub use types::{
    // Session
    Session, AccountData, CoreCapability,
    // Filter/Sort
    Filter, FilterOperator, Comparator,
    // Response types
    SetResponse, SetError, ChangesResponse, QueryChangesResponse, AddedItem,
    // Mailbox
    Mailbox, MailboxRights, MailboxFilterCondition,
    // Thread
    Thread,
    // Email
    Email, EmailAddress, BodyPart, EmailBodyPart, EmailHeader, EmailBodyValue,
    EmailFilterCondition, EmailCreate, EmailImport,
    // SearchSnippet
    SearchSnippet,
    // Identity
    Identity,
    // EmailSubmission
    EmailSubmission, Envelope, UndoStatus, DeliveryStatus,
    // VacationResponse
    VacationResponse,
    // Blob types (RFC 9404)
    BlobCapability, BlobUploadObject, DataSourceObject,
    BlobCreatedInfo, BlobUploadResponse, BlobGetResponse, BlobLookupInfo,
    BlobCopyResponse,
    // Sharing types (RFC 9670)
    Principal, PrincipalType, PrincipalFilterCondition, PrincipalSortProperty,
    ShareNotification, Entity,
    ShareNotificationFilterCondition, ShareNotificationSortProperty,
    PrincipalsCapability, PrincipalsAccountCapability, PrincipalsOwnerCapability,
    // Push
    PushSubscription, PushKeys,
};
```

---

### Phase 6: Tests and Verification

#### Task 16: Add Unit Tests

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add serialization tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox_filter_serialization() {
        let filter = MailboxFilterCondition {
            role: Some("inbox".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(filter).unwrap();
        assert_eq!(json, serde_json::json!({"role": "inbox"}));
    }

    #[test]
    fn test_email_filter_serialization() {
        let filter = EmailFilterCondition {
            in_mailbox: Some("inbox-id".to_string()),
            has_attachment: Some(true),
            ..Default::default()
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
```

---

#### Task 17: Build and Verify

**Step 1: Build workspace**

Run: `cargo build --workspace`
Expected: SUCCESS

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 3: Commit**

```bash
git add .
git commit -m "feat(jmap-client): complete RFC 8620 and RFC 8621 implementation"
```

---

## Completion Checklist

### RFC 8620 (JMAP Core)
- [ ] Session with all fields
- [ ] CoreCapability type
- [ ] Filter/Comparator generic types
- [ ] SetResponse/SetError types
- [ ] MethodError types
- [ ] Core/echo ✓ (existing)
- [ ] /changes pattern ✓ (existing)
- [ ] /queryChanges pattern ✓ (existing)
- [ ] PushSubscription ✓ (existing)
- [ ] Blob/copy ✓ (existing)

### RFC 8621 (JMAP Mail)
- [ ] Complete Mailbox type with rights
- [ ] Mailbox/query method
- [ ] Mailbox/set update
- [ ] Thread type and methods
- [ ] Complete Email type
- [ ] EmailFilterCondition
- [ ] Email/set create/update
- [ ] Email/copy
- [ ] Email/import
- [ ] Email/parse
- [ ] SearchSnippet type and method
- [ ] Identity type and methods
- [ ] EmailSubmission type and methods
- [ ] VacationResponse type and methods

---

**Estimated effort:** 17 tasks, ~4-6 hours total

**Execution:** Use `superpowers:executing-plans` or `superpowers:subagent-driven-development`
