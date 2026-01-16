// jmap-client/src/client.rs
use crate::http::HttpClient;
use crate::types::{Email, Mailbox};
use anyhow::Result;
use serde_json::json;

const CORE_CAPABILITY: &str = "urn:ietf:params:jmap:core";
const MAIL_CAPABILITY: &str = "urn:ietf:params:jmap:mail";

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
