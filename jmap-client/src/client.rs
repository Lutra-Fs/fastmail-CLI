// jmap-client/src/client.rs
use crate::http::HttpClient;
use crate::types::{Email, MaskedEmail, MaskedEmailState};
use anyhow::Result;
use serde_json::json;

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

        // JMAP response is an array of [name, arguments, tag] per RFC 8621
        // Check for error in first response's arguments
        if let Some(resp_array) = resp.as_array() {
            if let Some(first_resp) = resp_array.first() {
                if let Some(args) = first_resp.get(1) {
                    if let Some(err) = args.get("error") {
                        anyhow::bail!("JMAP error: {}", err);
                    }
                }
            }
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

        // Parse response array: [[method, args, tag]]
        let resp_array = resp
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: not an array"))?;

        let first_resp = resp_array
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty JMAP response"))?;

        let args = first_resp
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no arguments"))?;

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

        let resp = self.call("Email/get", params).await?;

        // Parse response array: [[method, args, tag]]
        let resp_array = resp
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: not an array"))?;

        let first_resp = resp_array
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty JMAP response"))?;

        let args = first_resp
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("Invalid JMAP response: no arguments"))?;

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

    /// List all masked emails
    pub async fn masked_email_get_all(&self) -> Result<Vec<MaskedEmail>> {
        let params = json!({
            "accountId": self.account_id,
            "ids": null,  // Get all
        });

        let resp = self.call("MaskedEmail/get", params).await?;

        // CORRECT: Parse as JMAP array [[method, args, tag]]
        // Extract the "list" field from the args (second element of first response)
        let list = if let Some(resp_array) = resp.as_array() {
            let args = resp_array
                .first()
                .and_then(|r| r.get(1));

            if let Some(args) = args {
                if let Some(list_val) = args.get("list").and_then(|l| l.as_array()) {
                    list_val.iter()
                        .map(|v| serde_json::from_value::<MaskedEmail>(v.clone()))
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

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

        // CORRECT: Parse as JMAP array, extract from "created"->"new"
        let created = if let Some(resp_array) = resp.as_array() {
            resp_array
                .first()
                .and_then(|r| r.get(1))
                .and_then(|args| args.get("created"))
                .and_then(|c| c.get("new"))
                .cloned()
        } else {
            return Err(anyhow::anyhow!("Invalid response format"));
        };

        let created_value = created.ok_or_else(|| anyhow::anyhow!("No created email in response"))?;
        let email: MaskedEmail = serde_json::from_value(created_value)?;
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
