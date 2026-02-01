// jmap-client/src/client.rs
use crate::blob;
use crate::http::HttpClient;
use crate::types::{
    BlobCopyResponse, BlobGetResponse, BlobLookupInfo, BlobUploadObject, BlobUploadResponse,
    ChangesResponse, Email, EmailCreate, EmailImport, EmailSubmission, Envelope, Identity,
    Mailbox, Principal, PrincipalFilterCondition, PushSubscription, QueryChangesResponse,
    SearchSnippet, ShareNotification, ShareNotificationFilterCondition, Thread,
    VacationResponse,
};
use anyhow::Result;
use serde_json::json;

const CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";
const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";
const BLOB_CAPABILITY: &str = "urn:ietf:params:jmap:blob";
const PRINCIPALS_CAPABILITY: &str = "urn:ietf:params:jmap:principals";
const SUBMISSION_CAPABILITY: &str = "urn:ietf:params:jmap:submission";
const VACATION_CAPABILITY: &str = "urn:ietf:params:jmap:vacationresponse";

#[derive(Debug, Clone)]
pub struct Invocation {
    pub name: String,
    pub args: serde_json::Value,
    pub tag: String,
}

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

    /// Perform a raw HTTP GET request (for RFC 8620 downloadUrl)
    pub async fn http_get(&self, url: &str) -> Result<Vec<u8>> {
        self.http
            .get(url, vec![])
            .await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))
    }

    /// Perform a raw HTTP POST request (for RFC 8620 uploadUrl)
    pub async fn http_post(&self, url: &str, data: Vec<u8>, content_type: &str) -> Result<Vec<u8>> {
        self.http
            .post_binary(url, data, content_type)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))
    }

    /// Upload binary data using RFC 8620 uploadUrl
    /// Returns the blobId
    pub async fn upload_blob(&self, _data: &[u8], _type_: &str) -> Result<String> {
        // This would require upload_url from Session, which we don't have here
        // For now, this is a placeholder - the actual implementation should be in the client
        // that has access to the Session object
        Err(anyhow::anyhow!(
            "upload_blob requires Session upload_url - implement in client layer"
        ))
    }

    /// Download binary data using RFC 8620 downloadUrl
    /// Returns UTF-8 string (replaces invalid sequences)
    pub async fn download_blob(&self, url: &str) -> Result<String> {
        let bytes = self.http_get(url).await?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Download binary data as raw bytes
    pub async fn download_blob_bytes(&self, url: &str) -> Result<Vec<u8>> {
        self.http_get(url).await
    }

    pub async fn call_method(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let using = [CORE_CAPABILITY, MAIL_CAPABILITY];
        self.call_method_with_using(&using, method, params).await
    }

    pub async fn call_method_with_using(
        &self,
        using: &[&str],
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let body = json!({
            "using": using,
            "methodCalls": [[method, params, "0"]],
        });

        let body_bytes = serde_json::to_vec(&body)?;
        let resp_bytes = self
            .http
            .post_json(&self.api_url, body_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))?;

        let resp: serde_json::Value = serde_json::from_slice(&resp_bytes)?;

        let responses = parse_method_responses(&resp)?;
        let first = responses
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty JMAP response"))?;

        if first.name == "error" {
            anyhow::bail!("JMAP error: {}", first.args);
        }

        if first.name != method {
            anyhow::bail!(
                "Unexpected JMAP response method: expected {}, got {}",
                method,
                first.name
            );
        }

        Ok(first.args.clone())
    }

    /// List emails with optional limit
    pub async fn email_query(&self, limit: usize) -> Result<Vec<String>> {
        let params = json!({
            "accountId": self.account_id,
            "limit": limit,
            "sort": [{"property": "receivedAt", "isAscending": false}]
        });

        let args = self.call_method("Email/query", params).await?;

        let ids_arr = args
            .get("ids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no ids"))?;

        let ids: Vec<String> = ids_arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

        Ok(ids)
    }

    /// List emails in a mailbox by ID with optional limit
    pub async fn email_query_in_mailbox(
        &self,
        mailbox_id: &str,
        limit: usize,
    ) -> Result<Vec<String>> {
        let params = json!({
            "accountId": self.account_id,
            "limit": limit,
            "filter": { "inMailbox": mailbox_id },
            "sort": [{"property": "receivedAt", "isAscending": false}]
        });

        let args = self.call_method("Email/query", params).await?;

        let ids_arr = args
            .get("ids")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no ids"))?;

        let ids: Vec<String> = ids_arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(String::from)
            .collect();

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

        let args = self.call_method("Email/get", params).await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// Get a single email by ID
    pub async fn get_email(&self, id: &str) -> Result<Email> {
        let emails = self.email_get(&[id.to_string()]).await?;
        emails
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Email not found: {}", id))
    }

    /// Get a single email by ID with body values (fetches actual email body content)
    pub async fn get_email_with_body(&self, id: &str) -> Result<Email> {
        // First get the email to find out what body parts exist
        let email = self.get_email(id).await?;

        // Note: Standard JMAP servers would populate bodyValues via Email/get
        // This base implementation just returns the email without body values
        // Subclasses like FastmailClient should override this to fetch via downloadUrl
        Ok(email)
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

        self.call_method("Email/set", params).await?;
        Ok(())
    }

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
        mailbox_ids: Option<std::collections::HashMap<String, bool>>,
        keywords: Option<std::collections::HashMap<String, bool>>,
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
        mailbox_ids: std::collections::HashMap<String, bool>,
    ) -> Result<std::collections::HashMap<String, Email>> {
        let create: std::collections::HashMap<String, serde_json::Value> = email_ids
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

    /// Parse a blob as an RFC 5322 message without storing it (RFC 8621 §4.9)
    #[allow(clippy::too_many_arguments)]
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

    /// List all mailboxes
    pub async fn mailbox_get_all(&self) -> Result<Vec<Mailbox>> {
        let params = json!({
            "accountId": self.account_id,
            "ids": null,
        });

        let args = self.call_method("Mailbox/get", params).await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// Create a mailbox
    pub async fn mailbox_create(&self, name: &str) -> Result<Mailbox> {
        let params = json!({
            "accountId": self.account_id,
            "create": {
                "new": {
                    "name": name
                }
            }
        });

        let args = self.call_method("Mailbox/set", params).await?;

        // Check for errors in notCreated
        if let Some(not_created) = args.get("notCreated") {
            if let Some(error) = not_created.get("new") {
                anyhow::bail!("Failed to create mailbox: {}", error);
            }
        }

        // Get the created mailbox - extract just the id since Fastmail doesn't return name
        let id = args
            .get("created")
            .and_then(|c| c.get("new"))
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("No created mailbox in response"))?;

        Ok(Mailbox {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: None,
            role: None,
            sort_order: 0,
            total_emails: 0,
            unread_emails: 0,
            total_threads: 0,
            unread_threads: 0,
            my_rights: None,
            is_subscribed: false,
        })
    }

    /// Delete a mailbox by ID
    pub async fn mailbox_delete(&self, id: &str) -> Result<()> {
        let params = json!({
            "accountId": self.account_id,
            "destroy": [id]
        });

        self.call_method("Mailbox/set", params).await?;
        Ok(())
    }

    /// Query Mailboxes with filter and sort (RFC 8621 §2.3)
    pub async fn mailbox_query(
        &self,
        filter: Option<crate::types::MailboxFilterCondition>,
        sort: Option<Vec<crate::types::Comparator>>,
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

    // RFC 8621 Thread methods (§3)

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

    // RFC 8621 SearchSnippet methods (§5)

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

    // RFC 8621 Identity methods (§6)

    /// Get all Identities (RFC 8621 §6.1)
    pub async fn identity_get_all(&self) -> Result<Vec<Identity>> {
        let params = json!({
            "accountId": self.account_id,
            "ids": null,
        });

        let using = [CORE_CAPABILITY, MAIL_CAPABILITY, SUBMISSION_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "Identity/get", params)
            .await?;

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
        let args = self
            .call_method_with_using(&using, "Identity/changes", params)
            .await?;
        serde_json::from_value(args).map_err(Into::into)
    }

    // RFC 8621 EmailSubmission methods (§7)

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
        let args = self
            .call_method_with_using(&using, "EmailSubmission/set", params)
            .await?;

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
        let args = self
            .call_method_with_using(&using, "EmailSubmission/get", params)
            .await?;

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
        self.call_method_with_using(&using, "EmailSubmission/set", params)
            .await?;
        Ok(())
    }

    // RFC 8621 VacationResponse methods (§8)

    /// Get VacationResponse (RFC 8621 §8.1)
    /// Note: There is only ever one VacationResponse per account with id "singleton"
    pub async fn vacation_response_get(&self) -> Result<VacationResponse> {
        let params = json!({
            "accountId": self.account_id,
            "ids": ["singleton"],
        });

        let using = [CORE_CAPABILITY, VACATION_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "VacationResponse/get", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid VacationResponse/get response: no list"))?;

        let first = list
            .first()
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
        self.call_method_with_using(&using, "VacationResponse/set", params)
            .await?;
        Ok(())
    }

    /// Upload blobs via Blob/upload
    pub async fn blob_upload(
        &self,
        create: std::collections::HashMap<String, BlobUploadObject>,
    ) -> Result<BlobUploadResponse> {
        let params = json!({
            "accountId": self.account_id,
            "create": create,
        });

        let using = [CORE_CAPABILITY, BLOB_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "Blob/upload", params)
            .await?;

        serde_json::from_value(args).map_err(Into::into)
    }

    /// Get blob data via Blob/get
    pub async fn blob_get(
        &self,
        ids: &[String],
        properties: Option<Vec<String>>,
        offset: Option<u64>,
        length: Option<u64>,
    ) -> Result<Vec<BlobGetResponse>> {
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
        if let Some(off) = offset {
            params["offset"] = json!(off);
        }
        if let Some(len) = length {
            params["length"] = json!(len);
        }

        let using = [CORE_CAPABILITY, BLOB_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "Blob/get", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Blob/get response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// Look up blob references via Blob/lookup
    pub async fn blob_lookup(
        &self,
        ids: &[String],
        type_names: &[String],
    ) -> Result<Vec<BlobLookupInfo>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let params = json!({
            "accountId": self.account_id,
            "ids": ids,
            "typeNames": type_names,
        });

        let using = [CORE_CAPABILITY, BLOB_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "Blob/lookup", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Blob/lookup response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// Get blob as text (returns error if not valid UTF-8)
    pub async fn blob_get_as_text(&self, id: &str) -> Result<String> {
        let results = self.blob_get(&[id.to_string()], None, None, None).await?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Blob not found"))?;

        if result.is_encoding_problem {
            anyhow::bail!("Blob data is not valid UTF-8");
        }

        result.as_text()
    }

    /// Get blob as base64
    pub async fn blob_get_as_base64(&self, id: &str) -> Result<String> {
        let results = self
            .blob_get(
                &[id.to_string()],
                Some(vec!["data:asBase64".to_string(), "size".to_string()]),
                None,
                None,
            )
            .await?;

        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Blob not found"))?;

        result
            .data_as_base64
            .ok_or_else(|| anyhow::anyhow!("No base64 data in response"))
    }

    /// Get blob as raw bytes
    pub async fn blob_get_bytes(&self, id: &str) -> Result<Vec<u8>> {
        let results = self.blob_get(&[id.to_string()], None, None, None).await?;
        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Blob not found"))?;
        result.as_bytes()
    }

    /// Upload text as a blob
    pub async fn blob_upload_text(&self, text: &str, type_: Option<&str>) -> Result<String> {
        let mut create = std::collections::HashMap::new();
        create.insert(
            "single".to_string(),
            BlobUploadObject {
                data: vec![blob::data_source_from_text(text)],
                type_: type_.map(|s| s.to_string()),
            },
        );

        let response = self.blob_upload(create).await?;
        let created = response
            .created
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Upload failed"))?;

        Ok(created.1.id)
    }

    /// Upload raw bytes as a blob
    pub async fn blob_upload_bytes(&self, bytes: &[u8], type_: Option<&str>) -> Result<String> {
        let mut create = std::collections::HashMap::new();
        create.insert(
            "single".to_string(),
            BlobUploadObject {
                data: vec![blob::data_source_from_bytes(bytes)],
                type_: type_.map(|s| s.to_string()),
            },
        );

        let response = self.blob_upload(create).await?;
        let created = response
            .created
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Upload failed"))?;

        Ok(created.1.id)
    }

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
        let args = self
            .call_method_with_using(&using, "Principal/get", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Principal/get response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

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
        let args = self
            .call_method_with_using(&using, "Principal/query", params)
            .await?;

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
        self.call_method_with_using(&using, "Principal/changes", params)
            .await
    }

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
        let args = self
            .call_method_with_using(&using, "ShareNotification/get", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid ShareNotification/get response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

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
        let args = self
            .call_method_with_using(&using, "ShareNotification/query", params)
            .await?;

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
        self.call_method_with_using(&using, "ShareNotification/changes", params)
            .await
    }

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
        self.call_method_with_using(&using, "ShareNotification/set", params)
            .await?;
        Ok(())
    }

    // RFC 8620 Core methods

    /// Core/echo - simple ping test (RFC 8620 §4)
    pub async fn core_echo(&self, data: serde_json::Value) -> Result<serde_json::Value> {
        let using = [CORE_CAPABILITY];
        self.call_method_with_using(&using, "Core/echo", data).await
    }

    // RFC 8620 Changes methods (§5.2)

    /// Get Email changes since a state (RFC 8620 §5.2)
    pub async fn email_changes(
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

        let args = self.call_method("Email/changes", params).await?;
        serde_json::from_value(args).map_err(Into::into)
    }

    /// Get Mailbox changes since a state (RFC 8620 §5.2)
    pub async fn mailbox_changes(
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

        let args = self.call_method("Mailbox/changes", params).await?;
        serde_json::from_value(args).map_err(Into::into)
    }

    // RFC 8620 QueryChanges method (§5.6)

    /// Get Email/queryChanges for incremental query sync (RFC 8620 §5.6)
    pub async fn email_query_changes(
        &self,
        since_query_state: &str,
        filter: Option<serde_json::Value>,
        sort: Option<Vec<serde_json::Value>>,
        max_changes: Option<usize>,
    ) -> Result<QueryChangesResponse> {
        let mut params = json!({
            "accountId": self.account_id,
            "sinceQueryState": since_query_state,
        });

        if let Some(f) = filter {
            params["filter"] = f;
        }
        if let Some(s) = sort {
            params["sort"] = json!(s);
        }
        if let Some(mc) = max_changes {
            params["maxChanges"] = json!(mc);
        }

        let args = self.call_method("Email/queryChanges", params).await?;
        serde_json::from_value(args).map_err(Into::into)
    }

    // RFC 8620 Blob/copy (§6.3)

    /// Copy blobs between accounts (RFC 8620 §6.3)
    pub async fn blob_copy(
        &self,
        from_account_id: &str,
        blob_ids: &[String],
    ) -> Result<BlobCopyResponse> {
        let params = json!({
            "fromAccountId": from_account_id,
            "accountId": self.account_id,
            "blobIds": blob_ids,
        });

        let using = [CORE_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "Blob/copy", params)
            .await?;
        serde_json::from_value(args).map_err(Into::into)
    }

    // RFC 8620 Push methods (§7.2)

    /// Get PushSubscriptions by IDs (RFC 8620 §7.2.1)
    pub async fn push_subscription_get(
        &self,
        ids: Option<&[String]>,
    ) -> Result<Vec<PushSubscription>> {
        let params = json!({
            "ids": ids,
        });

        let using = [CORE_CAPABILITY];
        let args = self
            .call_method_with_using(&using, "PushSubscription/get", params)
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid PushSubscription/get response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// Create or update PushSubscriptions (RFC 8620 §7.2.2)
    pub async fn push_subscription_set(
        &self,
        create: Option<std::collections::HashMap<String, serde_json::Value>>,
        update: Option<std::collections::HashMap<String, serde_json::Value>>,
        destroy: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        let mut params = json!({});

        if let Some(c) = create {
            params["create"] = json!(c);
        }
        if let Some(u) = update {
            params["update"] = json!(u);
        }
        if let Some(d) = destroy {
            params["destroy"] = json!(d);
        }

        let using = [CORE_CAPABILITY];
        self.call_method_with_using(&using, "PushSubscription/set", params)
            .await
    }
}

fn parse_method_responses(resp: &serde_json::Value) -> Result<Vec<Invocation>> {
    let obj = resp
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: not an object"))?;

    let responses = obj
        .get("methodResponses")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: missing methodResponses"))?;

    let mut invocations = Vec::with_capacity(responses.len());
    for item in responses {
        let arr = item
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: invalid invocation"))?;

        if arr.len() != 3 {
            return Err(anyhow::anyhow!(
                "Invalid JMAP response: invocation must have 3 elements"
            ));
        }

        let name = arr[0]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: method name not string"))?
            .to_string();
        let args = arr[1].clone();
        let tag = arr[2]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: tag not string"))?
            .to_string();

        invocations.push(Invocation { name, args, tag });
    }

    Ok(invocations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::HttpError;
    use async_trait::async_trait;

    struct MockHttpClient {
        response: Vec<u8>,
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn post_json(&self, _url: &str, _body: Vec<u8>) -> Result<Vec<u8>, HttpError> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_call_method_parses_method_responses() {
        let response = serde_json::json!({
            "methodResponses": [
                ["Email/query", {"ids": ["id1"]}, "0"]
            ],
            "sessionState": "state1"
        });

        let client = JmapClient::new(
            MockHttpClient {
                response: serde_json::to_vec(&response).unwrap(),
            },
            "https://example.com/jmap".to_string(),
            "acc1".to_string(),
        );

        let args = client
            .call_method("Email/query", serde_json::json!({"accountId": "acc1"}))
            .await
            .unwrap();

        assert_eq!(args["ids"], serde_json::json!(["id1"]));
    }
}
