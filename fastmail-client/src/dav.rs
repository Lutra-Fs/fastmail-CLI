// fastmail-client/src/dav.rs
use anyhow::Result;
use hyper::Uri;
use hyper::body::Incoming;
use hyper::Response;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use libdav::dav::{
    Delete, FindCollections, FoundCollection, GetProperty, ListResources, ListedResource,
    PutResource, WebDavClient,
};
use libdav::Depth;
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
/// Uses a simplified type to avoid complex generic constraints.
/// The inner client handles the actual DAV operations.
pub struct DavClient {
    /// The underlying libdav WebDavClient
    /// Boxed to avoid exposing complex generic types
    inner: Box<dyn DavClientInner>,
    base_url: String,
    token: String,
}

/// Trait to abstract over the complex WebDavClient generic type
#[async_trait::async_trait]
trait DavClientInner: Send + Sync {
    async fn list_resources(&self, href: &str) -> Result<Vec<ListedResource>>;
    async fn delete_resource(&self, href: &str) -> Result<()>;
    async fn put_resource(&self, href: &str, data: String, content_type: &str) -> Result<Option<String>>;
    async fn find_collections(&self, uri: &Uri) -> Result<Vec<FoundCollection>>;
    async fn get_property(&self, href: &str, property: &libdav::PropertyName<'_, '_>) -> Result<Option<String>>;
}

/// Concrete implementation of DavClientInner
struct DavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = Response<Incoming>> + Send + Sync + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    client: WebDavClient<C>,
}

#[async_trait::async_trait]
impl<C> DavClientInner for DavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = Response<Incoming>> + Send + Sync + 'static,
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
}

impl DavClient {
    /// Create a new DAV client from URL and credentials
    pub async fn new(base_url: String, _account_id: String, token: String) -> Result<Self> {
        // Build full service URL
        let service_url = format!("{}/", base_url.trim_end_matches('/'));

        // Create HTTPS connector
        let https_connector = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .build();

        // Build HTTP client with auth
        let https_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let https_client = AddAuthorization::bearer(https_client, &token);

        // Create libdav client
        let uri = service_url.parse()?;
        let client = WebDavClient::new(uri, https_client);

        let inner = Box::new(DavClientInnerImpl { client });

        Ok(Self {
            inner,
            base_url: service_url,
            token,
        })
    }

    /// List resources at a given path
    ///
    /// Note: libdav's ListResources uses Depth::One internally
    pub async fn list(&self, path: &str, _depth: u8) -> Result<Vec<DavResource>> {
        let href = self.build_href_str(path)?;

        let resources = self.inner.list_resources(&href).await?;

        Ok(resources.into_iter().map(DavResource::from).collect())
    }

    /// Get properties for a single resource
    pub async fn get_properties(&self, path: &str) -> Result<DavResource> {
        let href = self.build_href_str(path)?;

        let resources = self.inner.list_resources(&href).await?;

        resources
            .into_iter()
            .map(DavResource::from)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", path))
    }

    /// Create a collection (MKCOL)
    /// Note: libdav 0.10 doesn't have direct MKCOL support, so this is unimplemented
    pub async fn create_collection(&self, _path: &str) -> Result<()> {
        Err(anyhow::anyhow!(
            "Create collection not yet implemented - libdav 0.10 doesn't expose MKCOL"
        ))
    }

    /// Delete a resource
    pub async fn delete(&self, path: &str) -> Result<()> {
        let href = self.build_href_str(path)?;
        self.inner.delete_resource(&href).await
    }

    /// Upload/put a resource
    pub async fn put(&self, path: &str, content: &[u8], content_type: &str) -> Result<String> {
        let href = self.build_href_str(path)?;

        // Convert bytes to string for libdav
        let data = String::from_utf8(content.to_vec())
            .map_err(|e| anyhow::anyhow!("Content is not valid UTF-8: {}", e))?;

        let etag = self.inner.put_resource(&href, data, content_type).await?;

        Ok(etag.unwrap_or_default())
    }

    /// Get resource content
    /// Note: This returns empty vec since ListResources doesn't return actual content
    pub async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let href = self.build_href_str(path)?;

        let resources = self.inner.list_resources(&href).await?;

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
        self.inner.find_collections(&uri).await
    }

    /// Get a property for a resource
    pub async fn get_property(
        &self,
        path: &str,
        property_name: &libdav::PropertyName<'_, '_>,
    ) -> Result<Option<String>> {
        let href = self.build_href_str(path)?;
        self.inner.get_property(&href, property_name).await
    }

    /// Build href as String (libdav expects &str for most operations)
    fn build_href_str(&self, path: &str) -> Result<String> {
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
        let href = self.build_href_str(path)?;
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
    /// Get the base path for this service
    pub fn base_path(&self, account_id: &str) -> String {
        match self {
            DavService::Calendars => format!("/dav/calendars/user/{}/", account_id),
            DavService::AddressBooks => format!("/dav/addressbooks/user/{}/", account_id),
            DavService::Files => format!("/files/{}/", account_id),
        }
    }

    /// Convert u8 depth to libdav depth
    pub fn depth_from_u8(value: u8) -> Depth {
        match value {
            0 => Depth::Zero,
            1 => Depth::One,
            _ => Depth::Infinity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_base_path() {
        assert_eq!(
            DavService::Calendars.base_path("user123"),
            "/dav/calendars/user/user123/"
        );
        assert_eq!(
            DavService::AddressBooks.base_path("user123"),
            "/dav/addressbooks/user/user123/"
        );
        assert_eq!(DavService::Files.base_path("user123"), "/files/user123/");
    }

    #[test]
    fn test_depth_from_u8() {
        // Test that depth conversion works using PartialEq
        assert!(matches!(DavService::depth_from_u8(0), Depth::Zero));
        assert!(matches!(DavService::depth_from_u8(1), Depth::One));
        assert!(matches!(DavService::depth_from_u8(2), Depth::Infinity));
        assert!(matches!(DavService::depth_from_u8(99), Depth::Infinity));
    }
}
