// fastmail-client/src/dav.rs
use crate::config::Config;
use anyhow::Result;
use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use libdav::dav::{
    Delete, FindCollections, FoundCollection, GetProperty, ListResources, ListedResource,
    PutResource, WebDavClient,
};
use serde::{Deserialize, Serialize};
use tower_http::auth::AddAuthorization;

/// Generic DAV resource metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavResource {
    pub href: String,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub is_collection: bool,
}

impl From<ListedResource> for DavResource {
    fn from(resource: ListedResource) -> Self {
        Self {
            href: resource.href,
            content_type: resource.content_type,
            etag: resource.etag,
            is_collection: resource.resource_type.is_collection,
        }
    }
}

/// DAV client wrapper for Fastmail
///
/// Note: The type parameter is complex due to libdav's generic requirements.
/// We use a simplified Box-based trait object approach to hide this complexity.
pub struct DavClient {
    /// The underlying libdav WebDavClient (boxed to hide complex generics)
    client: Box<dyn DavClientInner>,
    /// Base URL for this service instance
    base_url: String,
}

/// Trait to abstract over the complex WebDavClient generic type.
/// This allows us to hide the complex type parameters from the public API.
#[async_trait::async_trait]
trait DavClientInner: Send + Sync {
    async fn list_resources(&self, href: &str) -> Result<Vec<ListedResource>>;
    async fn delete_resource(&self, href: &str) -> Result<()>;
    async fn put_resource(&self, href: &str, data: String, content_type: &str) -> Result<Option<String>>;
    async fn find_collections(&self, uri: &Uri) -> Result<Vec<FoundCollection>>;
    async fn get_property(&self, href: &str, property: &libdav::PropertyName<'_, '_>) -> Result<Option<String>>;
    fn clone_client(&self) -> Box<dyn DavClientInner>;
}

/// Concrete implementation of DavClientInner
struct DavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = http::Response<hyper::body::Incoming>> + Send + Sync + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    client: WebDavClient<C>,
}

impl<C> Clone for DavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = http::Response<hyper::body::Incoming>>
        + Send
        + Sync
        + Clone
        + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<C> DavClientInner for DavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = http::Response<hyper::body::Incoming>>
        + Send
        + Sync
        + Clone
        + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    async fn list_resources(&self, href: &str) -> Result<Vec<ListedResource>> {
        let response = self.client.request(ListResources::new(href)).await?;
        Ok(response.resources)
    }

    async fn delete_resource(&self, href: &str) -> Result<()> {
        self.client.request(Delete::new(href).force()).await?;
        Ok(())
    }

    async fn put_resource(&self, href: &str, data: String, content_type: &str) -> Result<Option<String>> {
        let response = self
            .client
            .request(PutResource::new(href).create(data, content_type))
            .await?;
        Ok(response.etag)
    }

    async fn find_collections(&self, uri: &Uri) -> Result<Vec<FoundCollection>> {
        let response = self.client.request(FindCollections::new(uri)).await?;
        Ok(response.collections)
    }

    async fn get_property(&self, href: &str, property: &libdav::PropertyName<'_, '_>) -> Result<Option<String>> {
        let response = self.client.request(GetProperty::new(href, property)).await?;
        Ok(response.value)
    }

    fn clone_client(&self) -> Box<dyn DavClientInner> {
        Box::new(self.clone())
    }
}

impl DavClient {
    /// Create a new DAV client from Fastmail config
    ///
    /// This method builds a DAV client for the specified service type.
    /// It uses the config's account_id and token for authentication.
    pub async fn from_config(config: &Config, service: DavService) -> Result<Self> {
        let base_url = match service {
            DavService::Calendars => config.get_caldav_url(),
            DavService::AddressBooks => config.get_carddav_url(),
            DavService::Files => config.get_webdav_url(),
        };

        let account_id = config
            .account_id
            .clone()
            .unwrap_or_else(|| "default".to_string());

        let service_url = match service {
            DavService::Calendars => format!("{}/dav/calendars/user/{}/", base_url, account_id),
            DavService::AddressBooks => {
                format!("{}/dav/addressbooks/user/{}/", base_url, account_id)
            }
            DavService::Files => format!("{}/files/{}/", base_url, account_id),
        };

        // Create HTTPS connector
        let https_connector = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .build();

        // Build HTTP client with bearer token auth
        let https_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let https_client = AddAuthorization::bearer(https_client, &config.token);

        // Create libdav client
        let uri = service_url.parse()?;
        let client = WebDavClient::new(uri, https_client);

        let inner: Box<dyn DavClientInner> = Box::new(DavClientInnerImpl { client });

        Ok(Self {
            client: inner,
            base_url: service_url,
        })
    }

    /// Get the underlying libdav client
    ///
    /// This returns a clone of the inner client for use with CalDAV/CardDAV.
    pub fn inner(&self) -> Box<dyn DavClientInner> {
        self.client.clone_client()
    }

    /// List resources at a given path
    pub async fn list(&self, path: &str, depth: u8) -> Result<Vec<DavResource>> {
        let href = self.build_href(path)?;

        // Note: libdav's ListResources hardcodes Depth::One
        // The depth parameter is kept for API compatibility but not fully utilized
        let _depth = depth; // Silenced unused warning

        let resources = self.client.list_resources(&href).await?;

        Ok(resources.into_iter().map(DavResource::from).collect())
    }

    /// Get properties for a single resource
    pub async fn get_properties(&self, path: &str) -> Result<DavResource> {
        let href = self.build_href(path)?;

        let resources = self.client.list_resources(&href).await?;

        resources
            .into_iter()
            .map(DavResource::from)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", path))
    }

    /// Create a collection (MKCOL)
    /// Note: libdav 0.10 doesn't have direct MKCOL support
    pub async fn create_collection(&self, _path: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "Create collection not yet implemented - libdav 0.10 doesn't expose MKCOL"
        ))
    }

    /// Delete a resource
    pub async fn delete(&self, path: &str) -> Result<()> {
        let href = self.build_href(path)?;
        self.client.delete_resource(&href).await
    }

    /// Upload/put a resource
    pub async fn put(&self, path: &str, content: &[u8], content_type: &str) -> Result<String> {
        let href = self.build_href(path)?;

        // Convert bytes to string for libdav
        let data = String::from_utf8(content.to_vec())
            .map_err(|e| anyhow::anyhow!("Content is not valid UTF-8: {}", e))?;

        let etag = self.client.put_resource(&href, data, content_type).await?;

        Ok(etag.unwrap_or_default())
    }

    /// Get resource content
    /// Note: Returns empty vec since ListResources doesn't return actual content
    pub async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let href = self.build_href(path)?;

        let resources = self.client.list_resources(&href).await?;

        let _resource = resources
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", path))?;

        // ListResources doesn't return actual content, just metadata
        Ok(vec![])
    }

    /// Copy a resource (not directly supported by libdav)
    pub async fn copy(&self, _from: &str, _to: &str, _overwrite: bool) -> Result<()> {
        Err(anyhow::anyhow!("Copy operation not yet implemented"))
    }

    /// Move a resource (not directly supported by libdav)
    pub async fn move_resource(&self, _from: &str, _to: &str, _overwrite: bool) -> Result<()> {
        Err(anyhow::anyhow!("Move operation not yet implemented"))
    }

    /// Find collections at a given path
    pub async fn find_collections(&self, path: &str) -> Result<Vec<FoundCollection>> {
        let uri = self.build_uri(path)?;
        self.client.find_collections(&uri).await
    }

    /// Get a property for a resource
    pub async fn get_property(
        &self,
        path: &str,
        property_name: &libdav::PropertyName<'_, '_>,
    ) -> Result<Option<String>> {
        let href = self.build_href(path)?;
        self.client.get_property(&href, property_name).await
    }

    /// Build href as String
    fn build_href(&self, path: &str) -> Result<String> {
        let path = path.trim_start_matches('/');

        let full = if path.is_empty() {
            self.base_url.clone()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        };

        Ok(full)
    }

    /// Build href as Uri (for operations that require Uri)
    fn build_uri(&self, path: &str) -> Result<Uri> {
        let href = self.build_href(path)?;
        Ok(href.parse()?)
    }
}

/// DAV service type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavService {
    Calendars,
    AddressBooks,
    Files,
}

impl DavService {
    /// Get the base path for this service (for URL construction)
    pub fn base_path(&self, account_id: &str) -> String {
        match self {
            DavService::Calendars => format!("/dav/calendars/user/{}/", account_id),
            DavService::AddressBooks => format!("/dav/addressbooks/user/{}/", account_id),
            DavService::Files => format!("/files/{}/", account_id),
        }
    }
}

/// Convert u8 depth to libdav depth
///
/// The orphan rule prevents implementing `From<u8>` for `libdav::Depth` directly.
/// Instead, use this helper function or call `.into()` on a `DepthValue` wrapper.
pub fn depth_from_u8(value: u8) -> libdav::Depth {
    match value {
        0 => libdav::Depth::Zero,
        1 => libdav::Depth::One,
        _ => libdav::Depth::Infinity,
    }
}

/// Wrapper type to enable From<u8> conversion for Depth
///
/// Due to Rust's orphan rule, we cannot implement `From<u8> for libdav::Depth`.
/// Use this wrapper: `let depth: Depth = DepthValue::from(5u8).into();`
pub struct DepthValue(pub u8);

impl From<u8> for DepthValue {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<DepthValue> for libdav::Depth {
    fn from(value: DepthValue) -> Self {
        depth_from_u8(value.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_from_u8() {
        // Test the helper function
        assert!(matches!(depth_from_u8(0), libdav::Depth::Zero));
        assert!(matches!(depth_from_u8(1), libdav::Depth::One));
        assert!(matches!(depth_from_u8(2), libdav::Depth::Infinity));
        assert!(matches!(depth_from_u8(99), libdav::Depth::Infinity));
    }

    #[test]
    fn test_depth_value_wrapper() {
        // Test the wrapper type that enables From<u8>
        let depth: libdav::Depth = DepthValue::from(0u8).into();
        assert!(matches!(depth, libdav::Depth::Zero));

        let depth: libdav::Depth = DepthValue::from(1u8).into();
        assert!(matches!(depth, libdav::Depth::One));

        let depth: libdav::Depth = DepthValue::from(99u8).into();
        assert!(matches!(depth, libdav::Depth::Infinity));
    }

    #[test]
    fn test_dav_service_paths() {
        let account_id = "testuser";
        let base_url = "https://dav.fastmail.com";

        assert_eq!(
            format!("{}{}", base_url, DavService::Calendars.base_path(account_id)),
            "https://dav.fastmail.com/dav/calendars/user/testuser/"
        );
        assert_eq!(
            format!("{}{}", base_url, DavService::AddressBooks.base_path(account_id)),
            "https://dav.fastmail.com/dav/addressbooks/user/testuser/"
        );
        assert_eq!(
            format!("{}{}", base_url, DavService::Files.base_path(account_id)),
            "https://dav.fastmail.com/files/testuser/"
        );
    }
}
