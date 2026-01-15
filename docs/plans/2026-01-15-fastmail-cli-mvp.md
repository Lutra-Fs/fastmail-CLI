# Fastmail CLI MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build an agent-first CLI for Fastmail with Email operations, Masked Email management, and safety-first design.

**Architecture:** Three-crates workspace: `jmap-client` (generic JMAP), `fastmail-client` (Fastmail-specific + libdav), `fastmail-cli` (CLI). Safety layers include `--force`/`--confirm` flags, recipient whitelist, and dry-run mode.

**Tech Stack:** Rust, clap (CLI), reqwest (HTTP), serde (JSON), libdav (WebDAV/CalDAV/CardDAV), directories (config)

---

## Task 1: Initialize Workspace and Project Structure

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `jmap-client/Cargo.toml`
- Create: `jmap-client/src/lib.rs`
- Create: `fastmail-client/Cargo.toml`
- Create: `fastmail-client/src/lib.rs`
- Create: `fastmail-cli/Cargo.toml`
- Create: `fastmail-cli/src/main.rs`

**Step 1: Create workspace Cargo.toml**

```toml
# Cargo.toml (root)
[workspace]
members = ["jmap-client", "fastmail-client", "fastmail-cli"]
resolver = "2"

[workspace.dependencies]
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = "0.1"
```

**Step 2: Create jmap-client crate**

```toml
# jmap-client/Cargo.toml
[package]
name = "jmap-client"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
async-trait = { workspace = true }
anyhow = { workspace = true }

[features]
default = ["reqwest"]
reqwest = ["dep:reqwest"]

[dependencies.reqwest]
version = "0.12"
optional = true
features = ["json"]
```

```rust
// jmap-client/src/lib.rs
pub mod client;
pub mod http;
pub mod types;

pub use client::JmapClient;
pub use http::HttpClient;
pub use types::{Email, EmailAddress};
```

**Step 3: Create fastmail-client crate**

```toml
# fastmail-client/Cargo.toml
[package]
name = "fastmail-client"
version = "0.1.0"
edition = "2021"

[dependencies]
jmap-client = { path = "../jmap-client", features = ["reqwest"] }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { version = "1.0", features = ["full"] }

# For Contacts/Calendar/Files
libdav = "0.10"

# For config
directories = "0.5"
toml = "0.8"
```

```rust
// fastmail-client/src/lib.rs
pub mod client;
pub mod config;

pub use client::FastmailClient;
```

**Step 4: Create fastmail-cli crate**

```toml
# fastmail-cli/Cargo.toml
[package]
name = "fastmail-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "fastmail"
path = "src/main.rs"

[dependencies]
fastmail-client = { path = "../fastmail-client" }
anyhow = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { version = "1.0", features = ["full"] }
clap = { version = "4.5", features = ["derive"] }
```

```rust
// fastmail-cli/src/main.rs
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fastmail CLI - MVP");
    Ok(())
}
```

**Step 5: Verify workspace builds**

Run: `cargo build --workspace`
Expected: SUCCESS with warnings about unused code

**Step 6: Commit**

```bash
git add .
git commit -m "feat: initialize workspace with three crates"
```

---

## Task 2: Implement Generic HTTP Client Trait

**Files:**
- Create: `jmap-client/src/http.rs`
- Create: `jmap-client/src/http/reqwest.rs`

**Step 1: Write HttpClient trait definition**

```rust
// jmap-client/src/http.rs
use async_trait::async_trait;

/// Error from HTTP request
#[derive(Debug)]
pub struct HttpError {
    pub status: Option<u16>,
    pub message: String,
}

/// Generic HTTP client trait - users can implement their own
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// POST JSON data to URL, return response bytes
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError>;
}
```

**Step 2: Write ReqwestClient implementation**

```rust
// jmap-client/src/http/reqwest.rs
use super::{HttpClient, HttpError};
use async_trait::async_trait;
use reqwest::Client as ReqwestClient;

pub struct ReqwestClient {
    inner: ReqwestClient,
    bearer_token: Option<String>,
}

impl ReqwestClient {
    pub fn new() -> Self {
        Self {
            inner: ReqwestClient::new(),
            bearer_token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.bearer_token = Some(token);
        self
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for ReqwestClient {
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        let mut req = self.inner.post(url);

        if let Some(token) = &self.bearer_token {
            req = req.bearer_auth(token);
        }

        let resp = req
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| HttpError {
                status: None,
                message: e.to_string(),
            })?;

        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| HttpError {
                status: Some(status),
                message: e.to_string(),
            })?
            .to_vec();

        if !resp.status().is_success() {
            return Err(HttpError {
                status: Some(status),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        Ok(bytes)
    }
}
```

**Step 3: Export in lib.rs**

```rust
// jmap-client/src/lib.rs
pub mod client;
pub mod http;
pub mod types;

pub use client::JmapClient;
pub use http::{HttpClient, HttpError};
pub use types::{Email, EmailAddress};

// Re-export reqwest client when feature is enabled
#[cfg(feature = "reqwest")]
pub use http::reqwest::ReqwestClient;
```

**Step 4: Build and verify**

Run: `cargo build -p jmap-client`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add jmap-client/src/
git commit -m "feat(jmap-client): add generic HttpClient trait with ReqwestClient"
```

---

## Task 3: Implement JMAP Types

**Files:**
- Create: `jmap-client/src/types.rs`

**Step 1: Write basic JMAP types**

```rust
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
    pub part_id: String,
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

#[derive(Debug, Deserialize)]
pub struct AccountData {
    #[serde(rename = "id")]
    pub account_id: String,
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
```

**Step 2: Build and verify**

Run: `cargo build -p jmap-client`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add jmap-client/src/types.rs
git commit -m "feat(jmap-client): add JMAP types (Email, Session, MaskedEmailState)"
```

---

## Task 4: Implement JMAP Client Core

**Files:**
- Create: `jmap-client/src/client.rs`

**Step 1: Write JmapClient struct and basic methods**

```rust
// jmap-client/src/client.rs
use crate::http::HttpClient;
use crate::types::{Email, Session};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

pub struct JmapClient<C: HttpClient> {
    http: C,
    api_url: String,
    account_id: String,
}

impl<C: HttpClient> JmapClient<C> {
    pub fn new(http: C, api_url: String, account_id: String) -> Self {
        Self {
            http,
            api_url,
            account_id,
        }
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Make a JMAP request
    async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let body = json!({
            "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
            "methodCalls": [[method, params, "0"]],
        });

        let body_bytes = serde_json::to_vec(&body)?;
        let resp_bytes = self
            .http
            .post_json(&self.api_url, body_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))?;

        let resp: serde_json::Value = serde_json::from_slice(&resp_bytes)?;

        // Check for JMAP errors
        if let Some(err) = resp["methodResponses"][0][1].get("error") {
            anyhow::bail!("JMAP error: {}", err);
        }

        Ok(resp)
    }

    /// List emails with optional limit
    pub async fn email_query(&self, limit: usize) -> Result<Vec<String>> {
        let params = json!({
            "accountId": self.account_id,
            "limit": limit,
            "sort": [{"property": "receivedAt", "isAscending": false}]
        });

        let resp = self.call("Email/query", params).await?;
        let ids: Vec<String> = resp["methodResponses"][0][1]["ids"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        Ok(ids)
    }

    /// Get emails by IDs
    pub async fn email_get(&self, ids: &[String]) -> Result<Vec<Email>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let params = json!({
            "accountId": self.account_id,
            "ids": ids,
        });

        let resp = self.call("Email/get", params).await?;
        let emails: Vec<Email> = resp["methodResponses"][0][1]["list"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(emails)
    }

    /// Get a single email by ID
    pub async fn get_email(&self, id: &str) -> Result<Email> {
        let emails = self.email_get(&[id.to_string()]).await?;
        emails
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Email not found: {}", id))
    }

    /// Delete emails by IDs
    pub async fn email_delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let params = json!({
            "accountId": self.account_id,
            "destroy": ids,
        });

        self.call("Email/set", params).await?;
        Ok(())
    }
}
```

**Step 2: Build and verify**

Run: `cargo build -p jmap-client`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add jmap-client/src/client.rs
git commit -m "feat(jmap-client): implement Email/query, Email/get, Email/set"
```

---

## Task 5: Implement Fastmail Client with Session Fetch

**Files:**
- Create: `fastmail-client/src/client.rs`

**Step 1: Write FastmailClient**

```rust
// fastmail-client/src/client.rs
use anyhow::Result;
use jmap_client::{HttpClient, JmapClient, ReqwestClient, Session};
use serde_json::Value;

const FASTMAIL_SESSION_URL: &str = "https://api.fastmail.com/jmap/session";

pub struct FastmailClient {
    inner: JmapClient<ReqwestClient>,
    account_email: String,
}

impl FastmailClient {
    pub async fn new(token: String) -> Result<Self> {
        let http_client = ReqwestClient::new().with_token(token.clone());

        // Fetch session from Fastmail
        let session = Self::fetch_session(&http_client).await?;

        // Parse account ID from session
        let account_id = session
            .accounts
            .first()
            .ok_or_else(|| anyhow::anyhow!("No account in session"))?
            .account_id
            .clone();

        let inner = JmapClient::new(http_client, session.api_url, account_id);

        // Get account email from session
        let account_email = Self::get_primary_account_email(&session).await?;

        Ok(Self {
            inner,
            account_email,
        })
    }

    async fn fetch_session(http: &ReqwestClient) -> Result<Session> {
        let body = b"";
        let resp_bytes = http
            .get(FASTMAIL_SESSION_URL, body.to_vec())
            .await
            .map_err(|e| anyhow!("Failed to fetch session: {}", e.message))?;

        let session: Session = serde_json::from_slice(&resp_bytes)?;
        Ok(session)
    }

    async fn get_primary_account_email(session: &Session) -> Result<String> {
        // For now, return a placeholder. In real implementation, we'd parse
        // the account data more carefully to get the actual email.
        Ok("user@fastmail.com".to_string())
    }

    pub fn account_id(&self) -> &str {
        self.inner.account_id()
    }

    pub fn account_email(&self) -> &str {
        &self.account_email
    }

    // Delegate to JmapClient

    pub async fn list_emails(&self, limit: usize) -> Result<Vec<Email>> {
        let ids = self.inner.email_query(limit).await?;
        self.inner.email_get(&ids).await
    }

    pub async fn get_email(&self, id: &str) -> Result<Email> {
        self.inner.get_email(id).await
    }

    pub async fn delete_emails(&self, ids: Vec<String>) -> Result<()> {
        self.inner.email_delete(&ids).await
    }
}
```

**Step 2: Add get method to HttpClient trait**

```rust
// jmap-client/src/http.rs - add get method
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError>;

    /// GET request for session
    async fn get(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        // Default implementation: same as POST (session endpoint accepts GET)
        self.post_json(url, body).await
    }
}
```

**Step 3: Implement get for ReqwestClient**

```rust
// jmap-client/src/http/reqwest.rs - add to impl
#[async_trait]
impl HttpClient for ReqwestClient {
    async fn post_json(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        // ... existing implementation ...
    }

    async fn get(&self, url: &str, _body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
        let mut req = self.inner.get(url);

        if let Some(token) = &self.bearer_token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await.map_err(|e| HttpError {
            status: None,
            message: e.to_string(),
        })?;

        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.map_err(|e| HttpError {
            status: Some(status),
            message: e.to_string(),
        })?.to_vec();

        if !resp.status().is_success() {
            return Err(HttpError {
                status: Some(status),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }

        Ok(bytes)
    }
}
```

**Step 4: Build and verify**

Run: `cargo build -p fastmail-client`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add fastmail-client/src/client.rs jmap-client/src/
git commit -m "feat(fastmail-client): add FastmailClient with session fetch"
```

---

## Task 6: Implement Config Storage

**Files:**
- Create: `fastmail-client/src/config.rs`

**Step 1: Write config module**

```rust
// fastmail-client/src/config.rs
use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    account: AccountConfig,
    #[serde(default)]
    safety: SafetyConfig,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AccountConfig {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SafetyConfig {
    #[serde(default = "default_require_new_recipient_flag")]
    pub require_new_recipient_flag: bool,
    #[serde(default = "default_require_confirm")]
    pub require_confirm: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_new_recipient_flag: true,
            require_confirm: true,
        }
    }
}

fn default_require_new_recipient_flag() -> bool {
    true
}

fn default_require_confirm() -> bool {
    true
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = Self::config_dir()?;

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            let default = Self::default();
            default.save()?;
            return Ok(default);
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.toml");

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        // Set permissions to 600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    fn config_dir() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "fastmail-cli", "fastmail-cli")
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?;
        Ok(proj_dirs.config_dir().to_path_buf())
    }

    pub fn account_email(&self) -> Option<&str> {
        self.account.email.as_deref()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            account: AccountConfig::default(),
            safety: SafetyConfig::default(),
        }
    }
}
```

**Step 2: Export in lib.rs**

```rust
// fastmail-client/src/lib.rs
pub mod client;
pub mod config;

pub use client::FastmailClient;
pub use config::Config;
```

**Step 3: Build and verify**

Run: `cargo build -p fastmail-client`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add fastmail-client/src/config.rs fastmail-client/src/lib.rs
git commit -m "feat(fastmail-client): add config storage with toml"
```

---

## Task 7: Implement Recipient Whitelist

**Files:**
- Create: `fastmail-client/src/whitelist.rs`

**Step 1: Write whitelist module**

```rust
// fastmail-client/src/whitelist.rs
use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Whitelist {
    pub allowed_recipients: Vec<String>,
}

impl Whitelist {
    pub fn load() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", "fastmail-cli", "fastmail-cli")
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?;

        let config_dir = proj_dirs.config_dir();
        let whitelist_path = config_dir.join("allowed-recipients.json");

        if !whitelist_path.exists() {
            fs::create_dir_all(config_dir)?;
            let default = Self::default();
            fs::write(&whitelist_path, serde_json::to_vec_pretty(&default)?)?;
            return Ok(default);
        }

        let content = fs::read_to_string(&whitelist_path)?;
        let whitelist: Whitelist = serde_json::from_str(&content)?;
        Ok(whitelist)
    }

    pub fn is_allowed(&self, email: &str) -> bool {
        self.allowed_recipients.iter().any(|r| r == email)
    }

    pub fn add(&mut self, email: String) -> Result<()> {
        if self.is_allowed(&email) {
            return Ok(());
        }
        self.allowed_recipients.push(email);
        self.save()
    }

    pub fn remove(&mut self, email: &str) -> Result<()> {
        self.allowed_recipients.retain(|r| r != email);
        self.save()
    }

    pub fn list(&self) -> &[String] {
        &self.allowed_recipients
    }

    fn save(&self) -> Result<()> {
        let proj_dirs = ProjectDirs::from("com", "fastmail-cli", "fastmail-cli")
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?;
        let whitelist_path = proj_dirs.config_dir().join("allowed-recipients.json");

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&whitelist_path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&whitelist_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&whitelist_path, perms)?;
        }

        Ok(())
    }
}

impl Default for Whitelist {
    fn default() -> Self {
        Self {
            allowed_recipients: Vec::new(),
        }
    }
}
```

**Step 2: Export in lib.rs**

```rust
// fastmail-client/src/lib.rs
pub mod client;
pub mod config;
pub mod whitelist;

pub use client::FastmailClient;
pub use config::Config;
pub use whitelist::Whitelist;
```

**Step 3: Build and verify**

Run: `cargo build -p fastmail-client`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add fastmail-client/src/whitelist.rs fastmail-client/src/lib.rs
git commit -m "feat(fastmail-client): add recipient whitelist for send safety"
```

---

## Task 8: Implement CLI Output Format

**Files:**
- Create: `fastmail-cli/src/output.rs`

**Step 1: Write output module with JSON envelope**

```rust
// fastmail-cli/src/output.rs
use serde::{Deserialize, Serialize};
use std::fmt;

/// Standard JSON response envelope
#[derive(Debug, Serialize)]
pub struct Response<T> {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
}

impl<T> Response<T> {
    pub fn ok(result: T) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
            meta: None,
        }
    }

    pub fn ok_with_meta(result: T, meta: Meta) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error: None,
            meta: Some(meta),
        }
    }

    pub fn error(error: ErrorResponse) -> Response<()> {
        Self {
            ok: false,
            result: None,
            error: Some(error),
            meta: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    #[serde(rename = "type")]
    type_: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retryable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<u64>,
}

impl ErrorResponse {
    pub fn safety_rejected(message: String) -> Self {
        Self {
            type_: "safety_rejected",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn not_found(message: String) -> Self {
        Self {
            type_: "not_found",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn validation_failed(message: String) -> Self {
        Self {
            type_: "validation_failed",
            message,
            retryable: Some(false),
            retry_after: None,
        }
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self {
            type_: "rate_limited",
            message: format!("Rate limited. Retry after {}s", retry_after),
            retryable: Some(true),
            retry_after: Some(retry_after),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limit: Option<RateLimitInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    remaining: u32,
    reset_at: String,
}

/// Exit codes for agent decision making
#[derive(Debug, Clone, Copy)]
pub enum ExitCode {
    Success = 0,
    TransientError = 1,
    PermanentError = 2,
    SafetyRejected = 3,
}

impl fmt::Display for ExitCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::TransientError => write!(f, "transient_error"),
            Self::PermanentError => write!(f, "permanent_error"),
            Self::SafetyRejected => write!(f, "safety_rejected"),
        }
    }
}

impl ExitCode {
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

// Print response to stdout
pub fn print_response<T: Serialize>(resp: &Response<T>) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(resp)?);
    Ok(())
}
```

**Step 2: Create mod.rs in CLI**

```rust
// fastmail-cli/src/lib.rs or add to main.rs for now
pub mod output;
```

**Step 3: Build and verify**

Run: `cargo build -p fastmail-cli`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add fastmail-cli/src/output.rs
git commit -m "feat(cli): add JSON envelope output format and exit codes"
```

---

## Task 9: Implement Basic CLI Structure

**Files:**
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Write full CLI structure**

```rust
// fastmail-cli/src/main.rs
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use output::{print_response, ErrorResponse, ExitCode, Meta, Response};

#[derive(Parser)]
#[command(name = "fastmail")]
#[command(about = "A command-line interface for Fastmail", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Email operations
    Mail(MailCommands),
    /// Masked email management
    Masked(MaskedCommands),
    /// Configuration
    Config(ConfigCommands),
}

#[derive(Subcommand)]
enum MailCommands {
    /// List emails
    List {
        /// Mailbox name [default: INBOX]
        #[arg(short, long)]
        mailbox: Option<String>,
        /// Max number of emails [default: 20]
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Read an email
    Read {
        /// Email ID
        id: String,
    },
    /// Delete emails
    Delete {
        /// Email ID(s) to delete
        #[arg(required = true)]
        ids: Vec<String>,
        /// Confirm destructive operation
        #[arg(long, required = true)]
        force: bool,
        /// Confirm intent (must contain email IDs)
        #[arg(long, required = true)]
        confirm: String,
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum MaskedCommands {
    /// List masked emails
    List {
        /// Filter by domain
        #[arg(short, long)]
        filter: Option<String>,
        /// Filter by state
        #[arg(short, long)]
        state: Option<String>,
    },
    /// Create a masked email
    Create {
        /// Domain for the masked email (e.g., https://example.com)
        domain: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Email prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },
    /// Enable a masked email
    Enable {
        /// Masked email ID or email address
        id: String,
    },
    /// Disable a masked email
    Disable {
        /// Masked email ID or email address
        id: String,
    },
    /// Delete a masked email
    Delete {
        /// Masked email ID or email address
        id: String,
        #[arg(long, required = true)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Manage recipient whitelist
    AllowRecipient(AllowRecipientCommands),
}

#[derive(Subcommand)]
enum AllowRecipientCommands {
    /// Add email to whitelist
    Add { email: String },
    /// List whitelist
    List,
    /// Remove email from whitelist
    Remove { email: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mail(mail) => handle_mail(mail).await,
        Commands::Masked(masked) => handle_masked(masked).await,
        Commands::Config(config) => handle_config(config).await,
    }
}

async fn handle_mail(cmd: MailCommands) -> Result<()> {
    match cmd {
        MailCommands::List { mailbox: _, limit } => {
            // Placeholder
            let resp = Response::ok_with_meta(
                vec![],
                Meta {
                    rate_limit: None,
                    dry_run: None,
                    operation_id: None,
                },
            );
            print_response(&resp)?;
            Ok(())
        }
        MailCommands::Read { id: _ } => {
            // Placeholder
            Ok(())
        }
        MailCommands::Delete {
            ids,
            force,
            confirm,
            dry_run,
        } => {
            // Safety check
            for id in &ids {
                if !confirm.contains(id) {
                    let resp = Response::error(ErrorResponse::safety_rejected(format!(
                        "--confirm must contain email ID '{}'. Use: --confirm 'delete-{}'",
                        id, id
                    )));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                // Show what would be deleted
                println!("Would delete: {:?}", ids);
                Ok(())
            } else {
                // Actually delete
                println!("Deleted: {:?}", ids);
                Ok(())
            }
        }
    }
}

async fn handle_masked(cmd: MaskedCommands) -> Result<()> {
    match cmd {
        MaskedCommands::List { .. } => {
            let resp = Response::ok(vec![]);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Create { .. } => Ok(()),
        MaskedCommands::Enable { .. } => Ok(()),
        MaskedCommands::Disable { .. } => Ok(()),
        MaskedCommands::Delete { .. } => Ok(()),
    }
}

async fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::AllowRecipient(allow) => match allow {
            AllowRecipientCommands::Add { email } => {
                println!("Added {} to whitelist", email);
                Ok(())
            }
            AllowRecipientCommands::List => {
                println!("Whitelist:");
                Ok(())
            }
            AllowRecipientCommands::Remove { email } => {
                println!("Removed {} from whitelist", email);
                Ok(())
            }
        },
    }
}
```

**Step 2: Test CLI structure**

Run: `cargo build -p fastmail-cli`
Then: `./target/debug/fastmail --help`
Expected: Show help with all commands

**Step 3: Commit**

```bash
git add fastmail-cli/src/main.rs
git commit -m "feat(cli): implement CLI structure with mail/masked/config commands"
```

---

## Task 10: Wire Up List Command with Real JMAP Client

**Files:**
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Add client loading logic**

```rust
// Add at top of main.rs
use fastmail_client::FastmailClient;
use std::env;

async fn load_client() -> Result<FastmailClient> {
    let token = env::var("FASTMAIL_TOKEN")
        .or_else(|_| -> Result<String> {
            Err(anyhow::anyhow!(
                "FASTMAIL_TOKEN environment variable not set"
            ))
        })?;

    FastmailClient::new(token).await
}
```

**Step 2: Implement list command**

```rust
// Replace MailCommands::List implementation
MailCommands::List { mailbox: _, limit } => {
    let client = load_client().await?;
    let emails = client.list_emails(limit).await?;

    let resp = Response::ok_with_meta(
        emails,
        Meta {
            rate_limit: None,  // TODO: track rate limits
            dry_run: None,
            operation_id: None,
        },
    );
    print_response(&resp)?;
    Ok(())
}
```

**Step 3: Build**

Run: `cargo build -p fastmail-cli`
Expected: SUCCESS

**Step 4: Manual test (requires real token)**

```bash
export FASTMAIL_TOKEN="your-token-here"
./target/debug/fastmail mail list --limit 5
```

Expected: JSON output with emails

**Step 5: Commit**

```bash
git add fastmail-cli/src/main.rs
git commit -m "feat(cli): wire up list command with real JMAP client"
```

---

## Task 11: Implement Masked Email Support in JMAP Client

**Files:**
- Modify: `jmap-client/src/types.rs`
- Modify: `jmap-client/src/client.rs`

**Step 1: Add MaskedEmail types**

```rust
// Add to jmap-client/src/types.rs

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MaskedEmailState {
    Pending,
    Enabled,
    Disabled,
    Deleted,
}
```

**Step 2: Add MaskedEmail methods to JmapClient**

```rust
// Add to jmap-client/src/client.rs

impl<C: HttpClient> JmapClient<C> {
    // ... existing methods ...

    /// List all masked emails
    pub async fn masked_email_get_all(&self) -> Result<Vec<MaskedEmail>> {
        let params = json!({
            "accountId": self.account_id,
            "ids": null,  // Get all
        });

        let resp = self.call("MaskedEmail/get", params).await?;
        let list: Vec<MaskedEmail> = resp["methodResponses"][0][1]["list"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(list)
    }

    /// Create a new masked email
    pub async fn masked_email_create(
        &self,
        for_domain: &str,
        description: &str,
        email_prefix: Option<&str>,
    ) -> Result<MaskedEmail> {
        let mut create_obj = json!({
            "forDomain": for_domain,
            "description": description,
        });

        if let Some(prefix) = email_prefix {
            create_obj["emailPrefix"] = json!(prefix);
        }

        let params = json!({
            "accountId": self.account_id,
            "create": {"new": create_obj},
        });

        let resp = self.call("MaskedEmail/set", params).await?;

        // Parse the created response
        let created = &resp["methodResponses"][0][1]["created"]["new"];
        let email: MaskedEmail = serde_json::from_value(created.clone())?;
        Ok(email)
    }

    /// Update masked email state
    pub async fn masked_email_set_state(
        &self,
        id: &str,
        state: MaskedEmailState,
    ) -> Result<()> {
        let params = json!({
            "accountId": self.account_id,
            "update": { id: { "state": serde_json::to_value(state)? } },
        });

        self.call("MaskedEmail/set", params).await?;
        Ok(())
    }

    /// Delete (set to deleted state) a masked email
    pub async fn masked_email_delete(&self, id: &str) -> Result<()> {
        self.masked_email_set_state(id, MaskedEmailState::Deleted)
            .await
    }
}
```

**Step 3: Export new types**

```rust
// jmap-client/src/lib.rs
pub use types::{Email, EmailAddress, MaskedEmail, MaskedEmailState};
```

**Step 4: Build**

Run: `cargo build -p jmap-client`
Expected: SUCCESS

**Step 5: Commit**

```bash
git add jmap-client/src/
git commit -m "feat(jmap-client): add MaskedEmail support (get/create/set state)"
```

---

## Task 12: Wire Up Masked Email Commands in CLI

**Files:**
- Modify: `fastmail-cli/src/main.rs`
- Modify: `fastmail-client/src/client.rs`

**Step 1: Delegate to FastmailClient**

```rust
// Add to fastmail-client/src/client.rs

use jmap_client::{MaskedEmail, MaskedEmailState};

impl FastmailClient {
    // ... existing methods ...

    pub async fn list_masked_emails(&self) -> Result<Vec<MaskedEmail>> {
        self.inner.masked_email_get_all().await
    }

    pub async fn create_masked_email(
        &self,
        for_domain: &str,
        description: &str,
        email_prefix: Option<&str>,
    ) -> Result<MaskedEmail> {
        self.inner
            .masked_email_create(for_domain, description, email_prefix)
            .await
    }

    pub async fn set_masked_email_state(
        &self,
        id: &str,
        state: MaskedEmailState,
    ) -> Result<()> {
        self.inner.masked_email_set_state(id, state).await
    }
}
```

**Step 2: Implement CLI commands**

```rust
// Replace handle_masked in fastmail-cli/src/main.rs

async fn handle_masked(cmd: MaskedCommands) -> Result<()> {
    let client = load_client().await?;

    match cmd {
        MaskedCommands::List { filter, state } => {
            let mut emails = client.list_masked_emails().await?;

            // Apply filters
            if let Some(domain) = filter {
                emails.retain(|e| e.for_domain.contains(&domain));
            }
            if let Some(state_str) = state {
                let state = match state_str.as_str() {
                    "pending" => MaskedEmailState::Pending,
                    "enabled" => MaskedEmailState::Enabled,
                    "disabled" => MaskedEmailState::Disabled,
                    "deleted" => MaskedEmailState::Deleted,
                    _ => return Err(anyhow::anyhow!("Invalid state: {}", state_str)),
                };
                emails.retain(|e| e.state == state);
            }

            let resp = Response::ok(emails);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Create {
            domain,
            description,
            prefix,
        } => {
            let email = client
                .create_masked_email(
                    &domain,
                    description.as_deref().unwrap_or(""),
                    prefix.as_deref(),
                )
                .await?;

            let resp = Response::ok(email);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Enable { id } => {
            client
                .set_masked_email_state(&id, MaskedEmailState::Enabled)
                .await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "enabled"}));
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Disable { id } => {
            client
                .set_masked_email_state(&id, MaskedEmailState::Disabled)
                .await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "disabled"}));
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Delete { id, force: _ } => {
            client.set_masked_email_state(&id, MaskedEmailState::Deleted).await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "deleted"}));
            print_response(&resp)?;
            Ok(())
        }
    }
}
```

**Step 3: Build**

Run: `cargo build -p fastmail-cli`
Expected: SUCCESS

**Step 4: Commit**

```bash
git add fastmail-client/src/client.rs fastmail-cli/src/main.rs
git commit -m "feat(cli): wire up masked email commands"
```

---

## Task 13: Implement Whitelist Commands

**Files:**
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Implement whitelist commands**

```rust
// Replace handle_config in fastmail-cli/src/main.rs

async fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::AllowRecipient(allow) => match allow {
            AllowRecipientCommands::Add { email } => {
                let mut whitelist = fastmail_client::Whitelist::load()?;
                whitelist.add(email.clone())?;
                let resp = Response::ok(serde_json::json!({
                    "email": email,
                    "added": true
                }));
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::List => {
                let whitelist = fastmail_client::Whitelist::load()?;
                let resp = Response::ok(serde_json::json!({
                    "allowed_recipients": whitelist.list()
                }));
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::Remove { email } => {
                let mut whitelist = fastmail_client::Whitelist::load()?;
                whitelist.remove(email.clone())?;
                let resp = Response::ok(serde_json::json!({
                    "email": email,
                    "removed": true
                }));
                print_response(&resp)?;
                Ok(())
            }
        },
    }
}
```

**Step 2: Build and test**

Run: `cargo build -p fastmail-cli`
Then: `./target/debug/fastmail config allow-recipient add test@example.com`
Expected: JSON response showing email was added

**Step 3: Commit**

```bash
git add fastmail-cli/src/main.rs
git commit -m "feat(cli): implement whitelist commands"
```

---

## Task 14: Implement Read Command

**Files:**
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Implement read command**

```rust
// Replace MailCommands::Read implementation
MailCommands::Read { id } => {
    let client = load_client().await?;
    let email = client.get_email(&id).await?;

    let resp = Response::ok(email);
    print_response(&resp)?;
    Ok(())
}
```

**Step 2: Build**

Run: `cargo build -p fastmail-cli`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add fastmail-cli/src/main.rs
git commit -m "feat(cli): implement read command"
```

---

## Task 15: Implement Delete Command with Dry-Run

**Files:**
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Implement delete with dry-run**

```rust
// Replace MailCommands::Delete implementation
MailCommands::Delete {
    ids,
    force,
    confirm,
    dry_run,
} => {
    // Safety check
    for id in &ids {
        if !confirm.contains(id) {
            let resp = Response::error(ErrorResponse::safety_rejected(format!(
                "--confirm must contain email ID '{}'. Use: --confirm 'delete-{}'",
                id, id
            )));
            print_response(&resp)?;
            std::process::exit(ExitCode::SafetyRejected.code());
        }
    }

    let client = load_client().await?;

    if dry_run {
        // Fetch emails that would be deleted
        let emails_to_delete = client.get_email(&ids[0]).await?;  // TODO: handle multiple

        let resp = Response::ok_with_meta(
            serde_json::json!({
                "operation": "delete",
                "would_delete": emails_to_delete
            }),
            Meta {
                rate_limit: None,
                dry_run: Some(true),
                operation_id: Some(format!("delete-{}", ids.join(","))),
            },
        );
        print_response(&resp)?;
        Ok(())
    } else {
        // Actually delete
        client.delete_emails(ids.clone()).await?;

        let resp = Response::ok_with_meta(
            serde_json::json!({
                "operation": "delete",
                "deleted": ids
            }),
            Meta {
                rate_limit: None,
                dry_run: Some(false),
                operation_id: Some(format!("delete-{}", ids.join(","))),
            },
        );
        print_response(&resp)?;
        Ok(())
    }
}
```

**Step 2: Build**

Run: `cargo build -p fastmail-cli`
Expected: SUCCESS

**Step 3: Commit**

```bash
git add fastmail-cli/src/main.rs
git commit -m "feat(cli): implement delete command with dry-run support"
```

---

## Task 16: Add README Documentation

**Files:**
- Create: `README.md`

**Step 1: Write README**

```markdown
# fastmail-cli

A command-line interface for Fastmail, designed for automation and AI agents.

## Features

- **Email operations**: list, read, delete
- **Masked Email**: full support for Fastmail's masked email feature
- **Agent-first**: JSON output, exit codes, and safety mechanisms
- **Safety-first**: `--force`, `--confirm`, whitelist, and dry-run modes

## Installation

```bash
cargo install --path .
```

## Configuration

Set your Fastmail API token:

```bash
export FASTMAIL_TOKEN="your-token-here"
```

Get a token at: https://app.fastmail.com/settings/security/integrations

## Usage

### List emails

```bash
fastmail mail list --limit 10
```

### Read an email

```bash
fastmail mail read <email-id>
```

### Delete emails (with safety checks)

```bash
# Preview what would be deleted
fastmail mail delete <id> --force --confirm "delete-<id>" --dry-run

# Actually delete
fastmail mail delete <id> --force --confirm "delete-<id>"
```

### Masked emails

```bash
# List all
fastmail masked list

# Create new
fastmail masked create https://example.com --description "Shopping site"

# Enable/disable
fastmail masked enable <id>
fastmail masked disable <id>

# Delete
fastmail masked delete <id> --force
```

### Whitelist (for send safety)

```bash
fastmail config allow-recipient add team@company.com
fastmail config allow-recipient list
fastmail config allow-recipient remove team@company.com
```

## Output Format

All commands output JSON:

```json
{
  "ok": true,
  "result": [...],
  "error": null,
  "meta": {
    "rate_limit": null,
    "dry_run": false,
    "operation_id": null
  }
}
```

## Exit Codes

- `0`: Success
- `1`: Transient error (retry safe)
- `2`: Permanent error (do not retry)
- `3`: Safety check failed (operation rejected)

## License

MIT OR Apache-2.0
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage examples"
```

---

## Completion Checklist

- [ ] Workspace builds successfully: `cargo build --workspace`
- [ ] All commands work: `fastmail mail list`, `fastmail masked list`, etc.
- [ ] JSON output format is consistent
- [ ] Safety checks enforce `--force` and `--confirm`
- [ ] Whitelist persists correctly
- [ ] Dry-run mode shows what would happen
- [ ] README documents all features

---

**Plan complete and saved to `docs/plans/2026-01-15-fastmail-cli-mvp.md`.**

Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

Which approach?
