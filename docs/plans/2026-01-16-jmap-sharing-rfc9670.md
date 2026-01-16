# JMAP Sharing Implementation Plan (RFC 9670)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement JMAP Sharing Extension (RFC 9670) for Principal and ShareNotification data types, enabling sharing and permission management in collaborative JMAP environments.

**Architecture:** Extend the three-crates workspace: add Principal and ShareNotification types to `jmap-client/src/types.rs`, add methods to `JmapClient` for Principal/ShareNotification operations, add capability detection to `FastmailClient`, and add CLI commands for sharing operations.

**Tech Stack:** Rust, serde (JSON), thiserror (errors), clap (CLI), tokio (async), chrono (UTCDate)

---

## Task 1: Add Chrono Dependency for UTCDate

**Files:**
- Modify: `jmap-client/Cargo.toml`

**Step 1: Add chrono dependency to jmap-client/Cargo.toml**

```toml
# jmap-client/Cargo.toml
[package]
name = "jmap-client"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = { workspace = true }
async-trait = "0.1"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"], optional = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = "1.0"
```

**Step 2: Run cargo check to verify dependencies**

Run: `cargo check -p jmap-client`
Expected: OK (dependencies resolve)

**Step 3: Commit**

```bash
git add jmap-client/Cargo.toml
git commit -m "feat(sharing): add chrono dependency for UTCDate"
```

---

## Task 2: Add Principal Type Definitions to types.rs

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add Principal types after Blob types**

```rust
// jmap-client/src/types.rs
// ... existing imports ...
use chrono::{DateTime, Utc};

/// JMAP Principals capability (urn:ietf:params:jmap:principals)
#[derive(Debug, Clone, Deserialize)]
pub struct PrincipalsCapability {
    /// Empty object for session-level capability
    #[serde(default)]
    pub _empty: serde_json::Value,
}

/// Account-level principals capability (urn:ietf:params:jmap:principals)
#[derive(Debug, Clone, Deserialize)]
pub struct PrincipalsAccountCapability {
    /// The id of the Principal that corresponds to the user fetching this object
    #[serde(rename = "currentUserPrincipalId")]
    pub current_user_principal_id: Option<String>,
}

/// Owner capability (urn:ietf:params:jmap:principals:owner)
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
    pub capabilities: std::collections::HashMap<String, serde_json::Value>,
    /// Map of Account id to Account object for each JMAP Account
    /// containing data for this Principal that the user has access to
    pub accounts: Option<std::collections::HashMap<String, AccountData>>,
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
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (types compile)

**Step 3: Commit**

```bash
git add jmap-client/src/types.rs
git commit -m "feat(sharing): add Principal type definitions per RFC 9670"
```

---

## Task 3: Add ShareNotification Type Definitions to types.rs

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add ShareNotification types after Principal types**

```rust
// jmap-client/src/types.rs
// After Principal types...

/// Entity that made a change (RFC 9670 Section 6)
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
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
    pub old_rights: Option<std::collections::HashMap<String, bool>>,
    /// The myRights property after the change
    #[serde(rename = "newRights")]
    pub new_rights: Option<std::collections::HashMap<String, bool>>,
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
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (types compile)

**Step 3: Commit**

```bash
git add jmap-client/src/types.rs
git commit -m "feat(sharing): add ShareNotification type definitions per RFC 9670"
```

---

## Task 4: Add Sharing Capability Constants to client.rs

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add sharing capability constants and imports**

```rust
// jmap-client/src/client.rs
use crate::types::{
    Email, Mailbox, BlobUploadObject, BlobUploadResponse, BlobGetResponse, BlobLookupInfo,
    Principal, ShareNotification,
    PrincipalFilterCondition, ShareNotificationFilterCondition,
    PrincipalSortProperty, ShareNotificationSortProperty,
};
use anyhow::Result;
use serde_json::json;

const CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";
const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";
const BLOB_CAPABILITY: &str = "urn:ietf:params:jmap:blob";
const PRINCIPALS_CAPABILITY: &str = "urn:ietf:params:jmap:principals";
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (constants added, unused imports OK for now)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add PRINCIPALS_CAPABILITY constant"
```

---

## Task 5: Add Principal/get Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add principal_get method**

```rust
// jmap-client/src/client.rs
// After blob_upload_bytes method:

/// Get Principals via Principal/get (RFC 9670)
pub async fn principal_get(
    &self,
    ids: &[String],
    properties: Option<Vec<String>>,
) -> Result<Vec<Principal>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = json!({
        "accountId": self.account_id,
        "ids": ids,
    });

    if let Some(props) = properties {
        params["properties"] = json!(props);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    let args = self.call_method_with_using(&using, "Principal/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid Principal/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (method compiles)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add principal_get method"
```

---

## Task 6: Add Principal/query Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add principal_query method**

```rust
// jmap-client/src/client.rs
// After principal_get method:

/// Query Principals via Principal/query (RFC 9670)
pub async fn principal_query(
    &self,
    filter: Option<PrincipalFilterCondition>,
    sort: Option<Vec<serde_json::Value>>,
    limit: Option<usize>,
) -> Result<Vec<String>> {
    let mut params = json!({
        "accountId": self.account_id,
    });

    if let Some(f) = filter {
        params["filter"] = serde_json::to_value(f)?;
    }
    if let Some(s) = sort {
        params["sort"] = json!(s);
    }
    if let Some(l) = limit {
        params["limit"] = json!(l);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    let args = self.call_method_with_using(&using, "Principal/query", params).await?;

    let ids_arr = args
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid Principal/query response: no ids"))?;

    let ids: Vec<String> = ids_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect();

    Ok(ids)
}

/// Query Principals and fetch full objects
pub async fn principal_query_and_get(
    &self,
    filter: Option<PrincipalFilterCondition>,
    sort: Option<Vec<serde_json::Value>>,
    limit: Option<usize>,
) -> Result<Vec<Principal>> {
    let ids = self.principal_query(filter, sort, limit).await?;
    self.principal_get(&ids, None).await
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (methods compile)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add principal_query and principal_query_and_get methods"
```

---

## Task 7: Add Principal/changes Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add principal_changes method**

```rust
// jmap-client/src/client.rs
// After principal_query_and_get method:

/// Get Principal changes via Principal/changes (RFC 9670)
pub async fn principal_changes(
    &self,
    since_state: String,
    max_changes: Option<usize>,
) -> Result<serde_json::Value> {
    let mut params = json!({
        "accountId": self.account_id,
        "sinceState": since_state,
    });

    if let Some(mc) = max_changes {
        params["maxChanges"] = json!(mc);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    self.call_method_with_using(&using, "Principal/changes", params).await
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (method compiles)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add principal_changes method"
```

---

## Task 8: Add ShareNotification/get Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add share_notification_get method**

```rust
// jmap-client/src/client.rs
// After principal_changes method:

/// Get ShareNotifications via ShareNotification/get (RFC 9670)
pub async fn share_notification_get(
    &self,
    ids: &[String],
    properties: Option<Vec<String>>,
) -> Result<Vec<ShareNotification>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut params = json!({
        "accountId": self.account_id,
        "ids": ids,
    });

    if let Some(props) = properties {
        params["properties"] = json!(props);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    let args = self.call_method_with_using(&using, "ShareNotification/get", params).await?;

    let list = args
        .get("list")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid ShareNotification/get response: no list"))?;

    list.iter()
        .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
        .collect()
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (method compiles)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add share_notification_get method"
```

---

## Task 9: Add ShareNotification/query Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add share_notification_query method**

```rust
// jmap-client/src/client.rs
// After share_notification_get method:

/// Query ShareNotifications via ShareNotification/query (RFC 9670)
pub async fn share_notification_query(
    &self,
    filter: Option<ShareNotificationFilterCondition>,
    sort: Option<Vec<serde_json::Value>>,
    limit: Option<usize>,
) -> Result<Vec<String>> {
    let mut params = json!({
        "accountId": self.account_id,
    });

    if let Some(f) = filter {
        params["filter"] = serde_json::to_value(f)?;
    }
    if let Some(s) = sort {
        params["sort"] = json!(s);
    }
    if let Some(l) = limit {
        params["limit"] = json!(l);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    let args = self.call_method_with_using(&using, "ShareNotification/query", params).await?;

    let ids_arr = args
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid ShareNotification/query response: no ids"))?;

    let ids: Vec<String> = ids_arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(String::from)
        .collect();

    Ok(ids)
}

/// Query ShareNotifications and fetch full objects
pub async fn share_notification_query_and_get(
    &self,
    filter: Option<ShareNotificationFilterCondition>,
    sort: Option<Vec<serde_json::Value>>,
    limit: Option<usize>,
) -> Result<Vec<ShareNotification>> {
    let ids = self.share_notification_query(filter, sort, limit).await?;
    self.share_notification_get(&ids, None).await
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (methods compile)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add share_notification_query methods"
```

---

## Task 10: Add ShareNotification/changes Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add share_notification_changes method**

```rust
// jmap-client/src/client.rs
// After share_notification_query_and_get method:

/// Get ShareNotification changes via ShareNotification/changes (RFC 9670)
pub async fn share_notification_changes(
    &self,
    since_state: String,
    max_changes: Option<usize>,
) -> Result<serde_json::Value> {
    let mut params = json!({
        "accountId": self.account_id,
        "sinceState": since_state,
    });

    if let Some(mc) = max_changes {
        params["maxChanges"] = json!(mc);
    }

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    self.call_method_with_using(&using, "ShareNotification/changes", params).await
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (method compiles)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add share_notification_changes method"
```

---

## Task 11: Add ShareNotification/set (destroy) Method to JmapClient

**Files:**
- Modify: `jmap-client/src/client.rs`

**Step 1: Add share_notification_destroy method**

```rust
// jmap-client/src/client.rs
// After share_notification_changes method:

/// Dismiss ShareNotifications via ShareNotification/set (RFC 9670)
/// Only destroy is supported for ShareNotifications
pub async fn share_notification_destroy(&self, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    let params = json!({
        "accountId": self.account_id,
        "destroy": ids,
    });

    let using = [CORE_CAPABILITY, PRINCIPALS_CAPABILITY];
    self.call_method_with_using(&using, "ShareNotification/set", params).await?;
    Ok(())
}
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (method compiles)

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(sharing): add share_notification_destroy method"
```

---

## Task 12: Export Sharing Types from jmap-client lib.rs

**Files:**
- Modify: `jmap-client/src/lib.rs`

**Step 1: Update lib.rs to export sharing types**

```rust
// jmap-client/src/lib.rs
pub mod blob;
pub mod client;
pub mod error;
pub mod http;
pub mod types;

pub use blob::{encode_base64, decode_base64, data_source_from_bytes, data_source_from_text};
pub use client::JmapClient;
pub use error::BlobError;
pub use http::HttpClient;
pub use types::{
    Email, EmailAddress, Mailbox, BodyPart, Session, AccountData,
    // Blob types
    BlobCapability, BlobUploadObject, DataSourceObject,
    BlobCreatedInfo, BlobUploadResponse,
    BlobGetResponse, BlobLookupInfo,
    // Sharing types (RFC 9670)
    Principal, PrincipalType, PrincipalFilterCondition, PrincipalSortProperty,
    ShareNotification, Entity,
    ShareNotificationFilterCondition, ShareNotificationSortProperty,
    PrincipalsCapability, PrincipalsAccountCapability, PrincipalsOwnerCapability,
};
```

**Step 2: Run cargo check**

Run: `cargo check -p jmap-client`
Expected: OK (exports compile)

**Step 3: Commit**

```bash
git add jmap-client/src/lib.rs
git commit -m "feat(sharing): export sharing types from lib.rs"
```

---

## Task 13: Add Sharing Capability Detection to FastmailClient

**Files:**
- Modify: `fastmail-client/src/client.rs`
- Modify: `fastmail-client/src/lib.rs`

**Step 1: Add sharing capability methods to FastmailClient**

```rust
// fastmail-client/src/client.rs
use jmap_client::{Email, HttpClient, JmapClient, Mailbox, ReqwestClient, Session};

// In impl FastmailClient block:

/// Check if server supports Principals capability
pub fn has_principals_capability(&self) -> bool {
    self.session
        .accounts
        .get(self.inner.account_id())
        .and_then(|acc| acc.account_capabilities.as_ref())
        .and_then(|caps| caps.get("urn:ietf:params:jmap:principals"))
        .is_some()
}

/// Get Principals capability details if available
pub fn principals_capability(&self) -> Option<jmap_client::PrincipalsAccountCapability> {
    use jmap_client::PrincipalsAccountCapability;
    self.session
        .accounts
        .get(self.inner.account_id())
        .and_then(|acc| acc.account_capabilities.as_ref())
        .and_then(|caps| caps.get("urn:ietf:params:jmap:principals"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Get owner capability (for finding principal account)
pub fn owner_capability(&self) -> Option<jmap_client::PrincipalsOwnerCapability> {
    use jmap_client::PrincipalsOwnerCapability;
    self.session
        .accounts
        .get(self.inner.account_id())
        .and_then(|acc| acc.account_capabilities.as_ref())
        .and_then(|caps| caps.get("urn:ietf:params:jmap:principals:owner"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Get current user's Principal ID
pub fn current_principal_id(&self) -> Option<String> {
    self.session
        .accounts
        .get(self.inner.account_id())
        .and_then(|acc| acc.account_capabilities.as_ref())
        .and_then(|caps| caps.get("urn:ietf:params:jmap:principals"))
        .and_then(|v| v.get("currentUserPrincipalId"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// List all Principals
pub async fn list_principals(
    &self,
    filter: Option<jmap_client::PrincipalFilterCondition>,
    limit: Option<usize>,
) -> Result<Vec<jmap_client::Principal>> {
    self.inner.principal_query_and_get(filter, None, limit).await
}

/// Get a specific Principal by ID
pub async fn get_principal(&self, id: &str) -> Result<jmap_client::Principal> {
    let results = self.inner.principal_get(&[id.to_string()], None).await?;
    results
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Principal not found: {}", id))
}

/// List ShareNotifications
pub async fn list_share_notifications(
    &self,
    filter: Option<jmap_client::ShareNotificationFilterCondition>,
    limit: Option<usize>,
) -> Result<Vec<jmap_client::ShareNotification>> {
    self.inner.share_notification_query_and_get(filter, None, limit).await
}

/// Dismiss ShareNotifications
pub async fn dismiss_share_notifications(&self, ids: &[String]) -> Result<()> {
    self.inner.share_notification_destroy(ids).await
}
```

**Step 2: Export sharing types from fastmail-client lib.rs**

```rust
// fastmail-client/src/lib.rs
pub use jmap_client::{
    // ... existing exports ...
    // Sharing types
    Principal, PrincipalType, PrincipalFilterCondition,
    ShareNotification, Entity,
    ShareNotificationFilterCondition,
    PrincipalsAccountCapability, PrincipalsOwnerCapability,
};
```

**Step 3: Run cargo check**

Run: `cargo check -p fastmail-client`
Expected: OK (methods compile)

**Step 4: Commit**

```bash
git add fastmail-client/src/client.rs fastmail-client/src/lib.rs
git commit -m "feat(sharing): add capability detection and helper methods"
```

---

## Task 14: Add Sharing Commands to CLI

**Files:**
- Create: `fastmail-cli/src/commands/sharing.rs`
- Modify: `fastmail-cli/src/commands/mod.rs`
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Create the sharing command module**

```rust
// fastmail-cli/src/commands/sharing.rs
use crate::output::{print_response, ErrorResponse, Meta, Response};
use anyhow::Result;
use fastmail_client::{
    Principal, PrincipalType, ShareNotification,
    PrincipalFilterCondition, ShareNotificationFilterCondition,
};

#[derive(clap::Subcommand, Clone, Debug)]
pub enum SharingCommands {
    /// Check principals capability
    Capability,
    /// List all principals
    ListPrincipals {
        /// Filter by name
        #[arg(short, long)]
        name: Option<String>,
        /// Filter by type
        #[arg(long)]
        type_: Option<String>,
        /// Limit results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get a specific principal
    GetPrincipal {
        /// Principal ID
        id: String,
    },
    /// List share notifications
    ListNotifications {
        /// Filter by object type
        #[arg(long)]
        object_type: Option<String>,
        /// Limit results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Dismiss share notifications
    DismissNotifications {
        /// Notification IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
    },
}

pub async fn handle_sharing_command(
    client: &fastmail_client::FastmailClient,
    cmd: SharingCommands,
) -> Result<()> {
    match cmd {
        SharingCommands::Capability => {
            if client.has_principals_capability() {
                let cap = client.principals_capability();
                let owner = client.owner_capability();
                let current_id = client.current_principal_id();
                let resp = Response::ok(serde_json::json!({
                    "supported": true,
                    "capability": cap,
                    "owner": owner,
                    "currentPrincipalId": current_id,
                }));
                print_response(&resp)?;
            } else {
                let resp = Response::ok(serde_json::json!({
                    "supported": false,
                }));
                print_response(&resp)?;
            }
            Ok(())
        }
        SharingCommands::ListPrincipals { name, type_, limit } => {
            let mut filter = PrincipalFilterCondition::default();

            if let Some(n) = name {
                filter.name = Some(n);
            }
            if let Some(t) = type_ {
                filter.type_ = match t.to_lowercase().as_str() {
                    "individual" => Some(PrincipalType::Individual),
                    "group" => Some(PrincipalType::Group),
                    "resource" => Some(PrincipalType::Resource),
                    "location" => Some(PrincipalType::Location),
                    "other" => Some(PrincipalType::Other),
                    _ => None,
                };
            }

            let principals = client
                .list_principals(Some(filter), limit)
                .await?;

            let resp = Response::ok(principals);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::GetPrincipal { id } => {
            let principal = client.get_principal(&id).await?;
            let resp = Response::ok(principal);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::ListNotifications { object_type, limit } => {
            let mut filter = ShareNotificationFilterCondition::default();

            if let Some(ot) = object_type {
                filter.object_type = Some(ot);
            }

            let notifications = client
                .list_share_notifications(Some(filter), limit)
                .await?;

            let resp = Response::ok(notifications);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::DismissNotifications { ids } => {
            if ids.is_empty() {
                let resp = Response::<()>::error(ErrorResponse::bad_request(
                    "No notification IDs provided".to_string()
                ));
                print_response(&resp)?;
                return Ok(());
            }

            client.dismiss_share_notifications(&ids).await?;

            let resp = Response::ok(serde_json::json!({
                "dismissed": ids,
            }));
            print_response(&resp)?;
            Ok(())
        }
    }
}
```

**Step 2: Update commands/mod.rs**

```rust
// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod config;
pub mod mail;
pub mod mailbox;
pub mod masked;
pub mod sharing;
```

**Step 3: Add Sharing variant to main.rs Commands enum**

```rust
// fastmail-cli/src/main.rs
// In Commands enum, add Sharing variant:

#[derive(Subcommand)]
enum Commands {
    /// Email operations
    #[command(subcommand)]
    Mail(MailCommands),
    /// Mailbox operations
    #[command(subcommand)]
    Mailbox(MailboxCommands),
    /// Masked email management
    #[command(subcommand)]
    Masked(MaskedCommands),
    /// Configuration
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Blob operations (JMAP RFC 9404)
    #[command(subcommand)]
    Blob(commands::blob::BlobCommands),
    /// Sharing operations (JMAP RFC 9670)
    #[command(subcommand)]
    Sharing(commands::sharing::SharingCommands),
}

// In main() match, add Sharing handler:
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mail(cmd) => handle_mail(cmd).await,
        Commands::Mailbox(cmd) => handle_mailbox(cmd).await,
        Commands::Masked(cmd) => handle_masked(cmd).await,
        Commands::Config(cmd) => handle_config(cmd).await,
        Commands::Blob(cmd) => handle_blob(cmd).await,
        Commands::Sharing(cmd) => handle_sharing(cmd).await,
    }
}

// Add handle_sharing function after handle_blob:
async fn handle_sharing(cmd: commands::sharing::SharingCommands) -> Result<()> {
    let client = load_client().await?;
    commands::sharing::handle_sharing_command(&client, cmd).await
}
```

**Step 4: Run cargo check**

Run: `cargo check -p fastmail-cli`
Expected: OK (CLI compiles)

**Step 5: Commit**

```bash
git add fastmail-cli/src/main.rs fastmail-cli/src/commands/
git commit -m "feat(sharing): add CLI commands for Sharing operations"
```

---

## Task 15: Add Unit Tests for Principal Types

**Files:**
- Modify: `jmap-client/src/types.rs`

**Step 1: Add tests module for Principal types**

```rust
// jmap-client/src/types.rs
// In #[cfg(test)] mod tests block:

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
```

**Step 2: Run tests**

Run: `cargo test -p jmap-client principal`
Expected: All tests pass

**Step 3: Commit**

```bash
git add jmap-client/src/types.rs
git commit -m "test(sharing): add unit tests for Principal types"
```

---

## Task 16: Update README with Sharing Usage Examples

**Files:**
- Modify: `README.md`

**Step 1: Add Sharing section to README**

```markdown
## Sharing Operations (JMAP RFC 9670)

Check if your account supports Sharing (Principals):

\`\`\`sh
fastmail sharing capability
\`\`\`

List all principals (users, groups, resources, locations):

\`\`\`sh
fastmail sharing list-principals
# Filter by name or type
fastmail sharing list-principals --name "John" --type individual
\`\`\`

Get a specific principal:

\`\`\`sh
fastmail sharing get-principal <PRINCIPAL_ID>
\`\`\`

List share notifications (permission changes):

\`\`\`sh
fastmail sharing list-notifications
# Filter by object type
fastmail sharing list-notifications --object-type Mailbox
\`\`\`

Dismiss share notifications:

\`\`\`sh
fastmail sharing dismiss-notifications --ids notification1,notification2,notification3
\`\`\`

> **Note:** Fastmail does not currently support the JMAP Sharing Extension (RFC 9670). This implementation exists for compatibility with other JMAP providers that support sharing and collaboration features.
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: OK

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs(sharing): add usage examples for Sharing operations"
```

---

## Task 17: Final Integration Build and Verification

**Files:**
- No file modifications

**Step 1: Full clean build**

Run:
```bash
cargo clean
cargo build --release
```
Expected: Clean build succeeds

**Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 3: Smoke test CLI commands**

Run:
```bash
./target/release/fastmail --help
./target/release/fastmail sharing --help
./target/release/fastmail sharing capability
```
Expected: Help displays, capability command works

**Step 4: Git tag for completion**

Run:
```bash
git tag -a v0.3.0 -m "Add JMAP Sharing support (RFC 9670)"
git push origin v0.3.0
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "release(sharing): v0.3.0 - JMAP Sharing support complete"
```

---

## Implementation Notes

**RFC 9670 Reference:** https://www.rfc-editor.org/rfc/rfc9670

**Key Implementation Decisions:**

1. **PrincipalType enum**: Uses serde(rename_all = "camelCase") to match RFC's lowercase values ("individual", "group", "resource", "location", "other")

2. **DateTime<Utc>**: Uses chrono crate for UTCDate type with serde feature for proper ISO 8601 serialization

3. **ShareNotification set**: Only destroy is supported; create/update are server-only and will be rejected by the server

4. **Filter conditions**: Default trait implementation for optional filters, making them easy to construct

5. **Capability detection**: Checks both session-level and account-level capabilities as per RFC

6. **CLI design**: Follows existing pattern with subcommands and JSON output envelope

**Testing Strategy:**

1. **Unit tests**: For serialization/deserialization and type correctness
2. **Manual integration tests**: For end-to-end CLI verification
3. **No mock tests**: Real server calls require JMAP provider with Sharing support

**Future Enhancements:**

1. Principal/set support for updating own principal details
2. ShareNotification queryChanges for incremental updates
3. Integration with shareable types (Mailbox, Calendar, etc.) to manage shareWith property
4. GUI for managing sharing permissions

**Fastmail Compatibility:**

> **Important:** Fastmail does not currently support the JMAP Sharing Extension (RFC 9670). The `sharing` commands will not work with Fastmail accounts. This implementation is provided for compatibility with other JMAP providers that implement RFC 9670 (e.g., some corporate JMAP servers, open-source implementations like JMAP-proxy, etc.).
