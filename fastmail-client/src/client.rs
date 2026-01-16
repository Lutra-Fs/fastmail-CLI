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
