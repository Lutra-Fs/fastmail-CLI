// fastmail-client/src/client.rs
use crate::masked_email::{MaskedEmail, MaskedEmailState};
use anyhow::{anyhow, Result};
use jmap_client::{Email, HttpClient, JmapClient, Mailbox, ReqwestClient, Session};
use serde_json::json;

const FASTMAIL_SESSION_URL: &str = "https://api.fastmail.com/jmap/session";
const FASTMAIL_MASKED_EMAIL_CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";
const JMAP_CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";

pub struct FastmailClient {
    inner: JmapClient<ReqwestClient>,
    account_email: String,
    session: Session,
}

impl FastmailClient {
    pub async fn new(token: String) -> Result<Self> {
        let http_client = ReqwestClient::new().with_token(token.clone());

        // Fetch session from Fastmail
        let session = Self::fetch_session(&http_client).await?;

        // Get account email from session first (before moving session)
        let account_email = Self::get_primary_account_email(&session).await?;

        // Parse account ID from session
        let account_id = Self::select_account_id(&session)?;

        let inner = JmapClient::new(http_client, session.api_url.clone(), account_id);

        Ok(Self {
            inner,
            account_email,
            session,
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

    async fn get_primary_account_email(_session: &Session) -> Result<String> {
        // For now, return a placeholder. In real implementation, we'd parse
        // the account data more carefully to get the actual email.
        Ok("user@fastmail.com".to_string())
    }

    fn select_account_id(session: &Session) -> Result<String> {
        if session.accounts.is_empty() {
            return Err(anyhow!("No account in session"));
        }

        if let Some((id, _)) = session
            .accounts
            .iter()
            .find(|(_, data)| data.is_personal.unwrap_or(false))
        {
            return Ok(id.clone());
        }

        let (id, _) = session
            .accounts
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No account in session"))?;
        Ok(id.clone())
    }

    pub fn account_id(&self) -> &str {
        self.inner.account_id()
    }

    pub fn account_email(&self) -> &str {
        &self.account_email
    }

    // Delegate to JmapClient

    pub async fn list_emails(&self, mailbox: Option<&str>, limit: usize) -> Result<Vec<Email>> {
        let ids = match mailbox {
            Some(name) => {
                let mailbox_id = self.resolve_mailbox_id(name).await?;
                self.inner.email_query_in_mailbox(&mailbox_id, limit).await?
            }
            None => self.inner.email_query(limit).await?,
        };
        self.inner.email_get(&ids).await
    }

    pub async fn get_email(&self, id: &str) -> Result<Email> {
        self.inner.get_email(id).await
    }

    /// Get email with body content included (Fastmail-specific implementation using RFC 8620 downloadUrl)
    pub async fn get_email_with_body(&self, id: &str) -> Result<Email> {
        // First get the email to find out what body parts exist
        let email = self.inner.get_email(id).await?;

        // Download the body content using RFC 8620 downloadUrl if available
        let body_values = if self.session.download_url.is_none() && self.session.upload_url.is_none() {
            None
        } else {
            let mut body_obj = serde_json::Map::new();

            // Download HTML body parts
            if let Some(html_body) = &email.html_body {
                for part in html_body {
                    if let Some(blob_id) = &part.blob_id {
                        let url = self.construct_download_url(blob_id, &part.type_);
                        let content = self.inner.download_blob(&url).await?;

                        let value_obj = json!({
                            "value": content,
                            "isEncodingProblem": false,
                            "isTruncated": false
                        });
                        body_obj.insert(part.part_id.clone(), value_obj);
                    }
                }
            }

            // Download text body parts
            if let Some(text_body) = &email.text_body {
                for part in text_body {
                    if let Some(blob_id) = &part.blob_id {
                        let url = self.construct_download_url(blob_id, &part.type_);
                        let content = self.inner.download_blob(&url).await?;

                        let value_obj = json!({
                            "value": content,
                            "isEncodingProblem": false,
                            "isTruncated": false
                        });
                        body_obj.insert(part.part_id.clone(), value_obj);
                    }
                }
            }

            if body_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(body_obj))
            }
        };

        Ok(Email {
            body_values,
            ..email
        })
    }

    /// Construct download URL from RFC 8620 downloadUrl template
    fn construct_download_url(&self, blob_id: &str, type_: &str) -> String {
        if let Some(download_url) = &self.session.download_url {
            download_url
                .replace("{accountId}", self.inner.account_id())
                .replace("{blobId}", blob_id)
                .replace("{name}", "email")
                .replace("{type}", type_)
        } else {
            // Fallback: construct URL directly (Fastmail-specific)
            format!(
                "https://www.fastmailusercontent.com/jmap/download/{}/{}?type={}",
                self.inner.account_id(),
                blob_id,
                type_
            )
        }
    }

    /// Upload binary data using RFC 8620 uploadUrl
    /// Returns the blobId
    pub async fn upload_blob(&self, data: &[u8], type_: &str) -> Result<String> {
        let upload_url = self.session.upload_url.as_ref()
            .ok_or_else(|| anyhow!("No uploadUrl in session"))?;

        let url = upload_url.replace("{accountId}", self.inner.account_id());

        // RFC 8620 says: POST with the file data as the body
        let resp_bytes = self.inner.http_post(&url, data.to_vec(), type_).await?;

        // Parse response to get blobId
        // Format: { "blobId": "xxx", "size": yyy }
        let resp: serde_json::Value = serde_json::from_slice(&resp_bytes)?;
        let blob_id = resp.get("blobId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No blobId in upload response"))?;

        Ok(blob_id.to_string())
    }

    /// Download a blob using RFC 8620 downloadUrl (private helper)
    async fn download_blob(&self, blob_id: &str, type_: &str) -> Result<String> {
        let url = self.construct_download_url(blob_id, type_);
        self.inner.download_blob(&url).await
    }

    /// Download a blob as bytes using RFC 8620 downloadUrl (private helper)
    async fn download_blob_bytes(&self, blob_id: &str, type_: &str) -> Result<Vec<u8>> {
        let url = self.construct_download_url(blob_id, type_);
        self.inner.download_blob_bytes(&url).await
    }

    pub async fn delete_emails(&self, ids: Vec<String>) -> Result<()> {
        self.inner.email_delete(&ids).await
    }

    pub async fn list_masked_emails(&self) -> Result<Vec<MaskedEmail>> {
        let args = self
            .inner
            .call_method_with_using(
                &[JMAP_CORE_CAPABILITY, FASTMAIL_MASKED_EMAIL_CAPABILITY],
                "MaskedEmail/get",
                json!({
                    "accountId": self.account_id(),
                    "ids": null,
                }),
            )
            .await?;

        let list = args
            .get("list")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Invalid JMAP response: no list"))?;

        list.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    pub async fn create_masked_email(
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

        let args = self
            .inner
            .call_method_with_using(
                &[JMAP_CORE_CAPABILITY, FASTMAIL_MASKED_EMAIL_CAPABILITY],
                "MaskedEmail/set",
                json!({
                    "accountId": self.account_id(),
                    "create": {"new": create_obj},
                }),
            )
            .await?;

        let created = args
            .get("created")
            .and_then(|c| c.get("new"))
            .cloned()
            .ok_or_else(|| anyhow!("No created email in response"))?;
        let email: MaskedEmail = serde_json::from_value(created)?;
        Ok(email)
    }

    pub async fn set_masked_email_state(
        &self,
        id: &str,
        state: MaskedEmailState,
    ) -> Result<()> {
        self.inner
            .call_method_with_using(
                &[JMAP_CORE_CAPABILITY, FASTMAIL_MASKED_EMAIL_CAPABILITY],
                "MaskedEmail/set",
                json!({
                    "accountId": self.account_id(),
                    "update": { id: { "state": serde_json::to_value(state)? } },
                }),
            )
            .await?;
        Ok(())
    }

    async fn resolve_mailbox_id(&self, mailbox_name: &str) -> Result<String> {
        let mailboxes = self.inner.mailbox_get_all().await?;
        let mailbox = mailboxes
            .into_iter()
            .find(|m| m.name == mailbox_name)
            .ok_or_else(|| anyhow!("Mailbox not found: {}", mailbox_name))?;
        Ok(mailbox.id)
    }

    pub async fn list_mailboxes(&self, filter: Option<&str>) -> Result<Vec<Mailbox>> {
        let mut mailboxes = self.inner.mailbox_get_all().await?;

        if let Some(pattern) = filter {
            let pattern_lower = pattern.to_lowercase();
            mailboxes.retain(|m| m.name.to_lowercase().contains(&pattern_lower));
        }

        Ok(mailboxes)
    }

    pub async fn create_mailbox(&self, name: &str) -> Result<Mailbox> {
        self.inner.mailbox_create(name).await
    }

    pub async fn delete_mailbox(&self, id: &str) -> Result<()> {
        self.inner.mailbox_delete(id).await
    }

    /// Check if server supports Blob capability
    pub fn has_blob_capability(&self) -> bool {
        self.session
            .accounts
            .get(self.inner.account_id())
            .and_then(|acc| acc.account_capabilities.as_ref())
            .and_then(|caps| caps.get("urn:ietf:params:jmap:blob"))
            .is_some()
    }

    /// Get Blob capability details if available
    pub fn blob_capability(&self) -> Option<jmap_client::BlobCapability> {
        self.session
            .accounts
            .get(self.inner.account_id())
            .and_then(|acc| acc.account_capabilities.as_ref())
            .and_then(|caps| caps.get("urn:ietf:params:jmap:blob"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get access to inner JmapClient for direct Blob operations
    pub fn jmap_client(&self) -> &JmapClient<ReqwestClient> {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastmail_session_url() {
        assert_eq!(FASTMAIL_SESSION_URL, "https://api.fastmail.com/jmap/session");
    }
}
