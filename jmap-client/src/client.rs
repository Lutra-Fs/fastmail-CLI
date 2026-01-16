// jmap-client/src/client.rs
use crate::blob;
use crate::http::HttpClient;
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
        self.http.get(url, vec![]).await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))
    }

    /// Perform a raw HTTP POST request (for RFC 8620 uploadUrl)
    pub async fn http_post(&self, url: &str, data: Vec<u8>, content_type: &str) -> Result<Vec<u8>> {
        self.http.post_binary(url, data, content_type).await
            .map_err(|e| anyhow::anyhow!("HTTP error: {}", e.message))
    }

    /// Upload binary data using RFC 8620 uploadUrl
    /// Returns the blobId
    pub async fn upload_blob(&self, data: &[u8], type_: &str) -> Result<String> {
        // This would require upload_url from Session, which we don't have here
        // For now, this is a placeholder - the actual implementation should be in the client
        // that has access to the Session object
        Err(anyhow::anyhow!("upload_blob requires Session upload_url - implement in client layer"))
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

    pub async fn call_method(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
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
    pub async fn email_query_in_mailbox(&self, mailbox_id: &str, limit: usize) -> Result<Vec<String>> {
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
        let args = self.call_method_with_using(&using, "Blob/upload", params).await?;

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
        let args = self.call_method_with_using(&using, "Blob/get", params).await?;

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
        let args = self.call_method_with_using(&using, "Blob/lookup", params).await?;

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

        result.data_as_base64
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
        let args = self.call_method_with_using(&using, "Principal/get", params).await?;

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
