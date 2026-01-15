// jmap-client/src/client.rs
use crate::http::HttpClient;

pub struct JmapClient<C: HttpClient> {
    http_client: C,
    api_url: String,
}

impl<C: HttpClient> JmapClient<C> {
    pub fn new(http_client: C, api_url: String) -> Self {
        Self {
            http_client,
            api_url,
        }
    }
}
