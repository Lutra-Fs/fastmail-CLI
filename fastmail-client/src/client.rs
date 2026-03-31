// fastmail-client/src/client.rs
use crate::masked_email::{MaskedEmail, MaskedEmailState};
use anyhow::{anyhow, Result};
use jmap_client::{Email, JmapClient, Mailbox, ReqwestClient};
use serde_json::json;

const FASTMAIL_SESSION_URL: &str = "https://api.fastmail.com/jmap/session";
const FASTMAIL_MASKED_EMAIL_CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";
const JMAP_CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";

pub struct FastmailClient {
    inner: JmapClient<ReqwestClient>,
    account_email: String,
}

impl FastmailClient {
    pub async fn new(token: String) -> Result<Self> {
        let inner = JmapClient::connect(FASTMAIL_SESSION_URL, token).await?;

        // Get account email from session
        let account_email = inner.account_email().unwrap_or_default().to_string();

        Ok(Self {
            inner,
            account_email,
        })
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
                self.inner
                    .email_query_in_mailbox(&mailbox_id, limit)
                    .await?
            }
            None => self.inner.email_query(limit).await?,
        };
        self.inner.email_get(&ids).await
    }

    pub async fn get_email(&self, id: &str) -> Result<Email> {
        self.inner.get_email(id).await
    }

    /// Get email with body content included (delegates to JmapClient)
    pub async fn get_email_with_body(&self, id: &str) -> Result<Email> {
        self.inner.get_email_with_body(id).await
    }

    /// Upload binary data using RFC 8620 uploadUrl (delegates to JmapClient)
    /// Returns the blobId
    pub async fn upload_blob(&self, data: &[u8], type_: &str) -> Result<String> {
        self.inner.upload_blob_content(data, type_).await
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

    pub async fn set_masked_email_state(&self, id: &str, state: MaskedEmailState) -> Result<()> {
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
        self.inner.has_capability("urn:ietf:params:jmap:blob")
    }

    /// Get Blob capability details if available
    pub fn blob_capability(&self) -> Option<jmap_client::BlobCapability> {
        self.inner
            .session()
            .accounts
            .get(self.inner.account_id())
            .and_then(|acc| acc.account_capabilities.as_ref())
            .and_then(|caps| caps.get("urn:ietf:params:jmap:blob"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    // Blob operations (RFC 9404) - forwarding methods to JmapClient

    /// Upload binary data as a blob
    /// Returns the blobId
    pub async fn blob_upload_bytes(&self, bytes: &[u8], type_: Option<&str>) -> Result<String> {
        self.inner.blob_upload_bytes(bytes, type_).await
    }

    /// Get blob content as raw bytes
    pub async fn blob_get_bytes(&self, id: &str) -> Result<Vec<u8>> {
        self.inner.blob_get_bytes(id).await
    }

    /// Get blob metadata
    pub async fn blob_get(
        &self,
        ids: &[String],
        properties: Option<Vec<String>>,
        offset: Option<u64>,
        length: Option<u64>,
    ) -> Result<Vec<jmap_client::BlobGetResponse>> {
        self.inner.blob_get(ids, properties, offset, length).await
    }

    /// Look up references to a blob
    pub async fn blob_lookup(
        &self,
        ids: &[String],
        type_names: &[String],
    ) -> Result<Vec<jmap_client::BlobLookupInfo>> {
        self.inner.blob_lookup(ids, type_names).await
    }

    // Sharing capability methods (RFC 9670)

    /// Check if server supports Principals capability
    pub fn has_principals_capability(&self) -> bool {
        self.inner.has_capability("urn:ietf:params:jmap:principals")
    }

    /// Get Principals capability details if available
    pub fn principals_capability(&self) -> Option<jmap_client::PrincipalsAccountCapability> {
        self.inner
            .session()
            .accounts
            .get(self.inner.account_id())
            .and_then(|acc| acc.account_capabilities.as_ref())
            .and_then(|caps| caps.get("urn:ietf:params:jmap:principals"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get owner capability (for finding principal account)
    pub fn owner_capability(&self) -> Option<jmap_client::PrincipalsOwnerCapability> {
        self.inner
            .session()
            .accounts
            .get(self.inner.account_id())
            .and_then(|acc| acc.account_capabilities.as_ref())
            .and_then(|caps| caps.get("urn:ietf:params:jmap:principals:owner"))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get current user's Principal ID
    pub fn current_principal_id(&self) -> Option<String> {
        self.inner
            .session()
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
        self.inner
            .principal_query_and_get(filter, None, limit)
            .await
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
        self.inner
            .share_notification_query_and_get(filter, None, limit)
            .await
    }

    /// Dismiss ShareNotifications
    pub async fn dismiss_share_notifications(&self, ids: &[String]) -> Result<()> {
        self.inner.share_notification_destroy(ids).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastmail_session_url() {
        assert_eq!(
            FASTMAIL_SESSION_URL,
            "https://api.fastmail.com/jmap/session"
        );
    }
}
