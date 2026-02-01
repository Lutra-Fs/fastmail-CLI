// fastmail-client/src/carddav.rs
//! CardDAV client implementation for contact operations.

use crate::config::Config;
use anyhow::{anyhow, Result};
use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use libdav::carddav::{CreateAddressBook, FindAddressBooks, GetAddressBookResources};
use libdav::dav::{Delete, FoundCollection, PutResource, WebDavClient};
use libdav::FetchedResource;
use serde::{Deserialize, Serialize};
use tower_http::auth::AddAuthorization;

/// A contact (VCARD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Unique identifier for the contact
    pub uid: String,
    /// First name
    #[serde(rename = "fn")]
    pub fn_: String,
    /// Last name
    #[serde(rename = "ln")]
    pub ln: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
    /// Organization
    pub organization: Option<String>,
    /// Job title
    pub title: Option<String>,
    /// Nickname
    pub nickname: Option<String>,
    /// Notes
    pub notes: Option<String>,
    /// Birthday (YYYY-MM-DD format)
    pub birthday: Option<String>,
    /// Website URL
    pub url: Option<String>,
}

/// An address book collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    /// Href (path) to the address book
    pub href: String,
    /// Display name for the address book
    pub display_name: String,
    /// Description of the address book
    pub description: Option<String>,
}

impl From<FoundCollection> for AddressBook {
    fn from(collection: FoundCollection) -> Self {
        // Extract name from href path (last segment without trailing slash)
        let display_name = collection
            .href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("Unnamed")
            .to_string();

        Self {
            href: collection.href,
            display_name,
            description: None,
        }
    }
}

/// CardDAV client wrapper
///
/// Wraps the libdav CardDavClient with a simplified API specific to Fastmail.
pub struct CardDavClient {
    /// The underlying libdav CardDavClient (boxed to hide complex generics)
    carddav: Box<dyn CardDavClientInner>,
    /// Base URL for CardDAV operations
    base_url: String,
}

/// Trait to abstract over the complex CardDavClient generic type.
#[async_trait::async_trait]
trait CardDavClientInner: Send + Sync {
    async fn find_address_books(&self, home_set: &Uri) -> Result<Vec<FoundCollection>>;
    async fn get_addressbook_resources(&self, href: &str) -> Result<Vec<FetchedResource>>;
    async fn delete_resource(&self, href: &str) -> Result<()>;
    async fn put_resource(
        &self,
        href: &str,
        data: String,
        content_type: &str,
    ) -> Result<Option<String>>;
    async fn create_address_book(&self, href: &str, display_name: &str) -> Result<()>;
}

/// Concrete implementation of CardDavClientInner
struct CardDavClientInnerImpl<C>
where
    C: tower_service::Service<
            http::Request<String>,
            Response = http::Response<hyper::body::Incoming>,
        > + Send
        + Sync
        + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    client: libdav::CardDavClient<C>,
}

#[async_trait::async_trait]
impl<C> CardDavClientInner for CardDavClientInnerImpl<C>
where
    C: tower_service::Service<
            http::Request<String>,
            Response = http::Response<hyper::body::Incoming>,
        > + Send
        + Sync
        + Clone
        + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    async fn find_address_books(&self, home_set: &Uri) -> Result<Vec<FoundCollection>> {
        let response = self.client.request(FindAddressBooks::new(home_set)).await?;
        Ok(response.addressbooks)
    }

    async fn get_addressbook_resources(&self, href: &str) -> Result<Vec<FetchedResource>> {
        let response = self
            .client
            .request(GetAddressBookResources::new(href))
            .await?;
        Ok(response.resources)
    }

    async fn delete_resource(&self, href: &str) -> Result<()> {
        self.client.request(Delete::new(href).force()).await?;
        Ok(())
    }

    async fn put_resource(
        &self,
        href: &str,
        data: String,
        content_type: &str,
    ) -> Result<Option<String>> {
        let response = self
            .client
            .request(PutResource::new(href).create(data, content_type))
            .await?;
        Ok(response.etag)
    }

    async fn create_address_book(&self, href: &str, display_name: &str) -> Result<()> {
        let create_address_book = CreateAddressBook::new(href).with_display_name(display_name);
        self.client.request(create_address_book).await?;
        Ok(())
    }
}

impl CardDavClient {
    /// Create a new CardDAV client from Fastmail config
    pub async fn from_config(config: &Config) -> Result<Self> {
        let base_url = config.get_carddav_url();
        let dav_username = config.get_dav_username()?;

        let service_url = format!("{}/dav/addressbooks/user/{}/", base_url, dav_username);

        // Create HTTPS connector
        let https_connector = HttpsConnectorBuilder::new()
            .with_native_roots()?
            .https_or_http()
            .enable_http1()
            .build();

        // Build HTTP client with DAV Basic auth
        // DAV endpoints use HTTP Basic Auth with email as username and app password as password
        let dav_username = config.get_dav_username()?;
        let dav_password = config.dav_password.as_ref()
            .ok_or_else(|| anyhow::anyhow!("DAV password not set. Please set FASTMAIL_DAV_PASSWORD environment variable with your Fastmail app password. Generate one at: https://www.fastmail.com/settings/passwords"))?;
        let https_client = Client::builder(TokioExecutor::new()).build(https_connector);
        let https_client = AddAuthorization::basic(https_client, dav_username, dav_password);

        // Create libdav CardDavClient
        let uri: Uri = service_url.parse()?;
        let webdav_client = WebDavClient::new(uri, https_client);
        let client = libdav::CardDavClient::new(webdav_client);

        let inner: Box<dyn CardDavClientInner> = Box::new(CardDavClientInnerImpl { client });

        Ok(Self {
            carddav: inner,
            base_url: service_url,
        })
    }

    /// List all address books for the user
    pub async fn list_address_books(&self) -> Result<Vec<AddressBook>> {
        let home_set: Uri = self.base_url.parse()?;
        let collections = self.carddav.find_address_books(&home_set).await?;

        Ok(collections.into_iter().map(AddressBook::from).collect())
    }

    /// Get a specific address book by href
    pub async fn get_address_book(&self, href: &str) -> Result<AddressBook> {
        let address_books = self.list_address_books().await?;

        address_books
            .into_iter()
            .find(|ab| ab.href == href || ab.href.ends_with(href))
            .ok_or_else(|| anyhow!("Address book not found: {}", href))
    }

    /// Create a new address book
    pub async fn create_address_book(
        &self,
        name: &str,
        description: Option<String>,
    ) -> Result<AddressBook> {
        let href = format!("{}{}/", self.base_url.trim_end_matches('/'), name);

        self.carddav.create_address_book(&href, name).await?;

        Ok(AddressBook {
            href: href.clone(),
            display_name: name.to_string(),
            description,
        })
    }

    /// Delete an address book
    pub async fn delete_address_book(&self, href: &str) -> Result<()> {
        self.carddav.delete_resource(href).await
    }

    /// List all contacts in an address book
    pub async fn list_contacts(&self, addressbook_href: &str) -> Result<Vec<Contact>> {
        let resources = self
            .carddav
            .get_addressbook_resources(addressbook_href)
            .await?;

        let mut contacts = Vec::new();
        for resource in resources {
            if let Ok(content) = resource.content {
                if let Some(contact) = Self::parse_vcard(content.data.as_bytes()) {
                    contacts.push(contact);
                }
            }
        }

        Ok(contacts)
    }

    /// Get a specific contact by href
    pub async fn get_contact(&self, contact_href: &str) -> Result<Contact> {
        let resources = self.carddav.get_addressbook_resources(contact_href).await?;

        resources
            .into_iter()
            .find(|r| r.href == contact_href || r.href.ends_with(contact_href))
            .and_then(|r| r.content.ok())
            .and_then(|c| Self::parse_vcard(c.data.as_bytes()))
            .ok_or_else(|| anyhow!("Contact not found or could not be parsed: {}", contact_href))
    }

    /// Create or update a contact in an address book
    pub async fn put_contact(&self, addressbook_href: &str, contact: &Contact) -> Result<String> {
        // Generate href from UID
        let contact_href = format!(
            "{}/{}.vcf",
            addressbook_href.trim_end_matches('/'),
            contact.uid
        );
        let vcard = Self::serialize_vcard(contact)?;

        let etag = self
            .carddav
            .put_resource(&contact_href, vcard, "text/vcard")
            .await?;

        Ok(etag.unwrap_or_default())
    }

    /// Delete a contact
    pub async fn delete_contact(&self, contact_href: &str) -> Result<()> {
        self.carddav.delete_resource(contact_href).await
    }

    /// Search contacts by query string
    pub async fn search_contacts(
        &self,
        addressbook_href: &str,
        query: &str,
    ) -> Result<Vec<Contact>> {
        let all_contacts = self.list_contacts(addressbook_href).await?;
        let query_lower = query.to_lowercase();

        let filtered = all_contacts
            .into_iter()
            .filter(|c| {
                c.fn_.to_lowercase().contains(&query_lower)
                    || c.ln
                        .as_ref()
                        .is_some_and(|ln| ln.to_lowercase().contains(&query_lower))
                    || c.email
                        .as_ref()
                        .is_some_and(|e| e.to_lowercase().contains(&query_lower))
                    || c.organization
                        .as_ref()
                        .is_some_and(|o| o.to_lowercase().contains(&query_lower))
                    || c.notes
                        .as_ref()
                        .is_some_and(|n| n.to_lowercase().contains(&query_lower))
            })
            .collect();

        Ok(filtered)
    }

    /// Parse a vCard from bytes (simplified vCard 3.0 MVP implementation)
    fn parse_vcard(data: &[u8]) -> Option<Contact> {
        let content = String::from_utf8(data.to_vec()).ok()?;

        // Very simplified vCard parser - just extracts basic fields
        let mut uid = None;
        let mut fn_ = None;
        let mut ln = None;
        let mut email = None;
        let mut phone = None;
        let mut organization = None;
        let mut title = None;
        let mut nickname = None;
        let mut notes = None;
        let mut birthday = None;
        let mut url = None;

        let mut in_vcard = false;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("BEGIN:VCARD") {
                in_vcard = true;
                continue;
            }

            if line.starts_with("END:VCARD") {
                break;
            }

            if !in_vcard {
                continue;
            }

            // Simple key-value parsing (ignoring folded lines for MVP)
            if let Some((key, value)) = line.split_once(':') {
                match key {
                    "UID" => uid = Some(value.to_string()),
                    "FN" => fn_ = Some(value.to_string()),
                    "N" => {
                        // N field: Family;Given;Additional;Prefix;Suffix
                        let parts: Vec<&str> = value.split(';').collect();
                        if parts.len() >= 2 {
                            ln = if !parts[0].is_empty() {
                                Some(parts[0].to_string())
                            } else {
                                None
                            };
                        }
                    }
                    "EMAIL" => email = Some(value.to_string()),
                    "TEL" => phone = Some(value.to_string()),
                    "ORG" => organization = Some(value.to_string()),
                    "TITLE" => title = Some(value.to_string()),
                    "NICKNAME" => nickname = Some(value.to_string()),
                    "NOTE" => notes = Some(value.to_string()),
                    "BDAY" => birthday = Some(value.to_string()),
                    "URL" => url = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        // Validate required fields
        let uid = uid?;
        let fn_ = fn_?;

        Some(Contact {
            uid,
            fn_,
            ln,
            email,
            phone,
            organization,
            title,
            nickname,
            notes,
            birthday,
            url,
        })
    }

    /// Serialize a contact to vCard format (simplified vCard 3.0 MVP implementation)
    fn serialize_vcard(contact: &Contact) -> Result<String> {
        let mut vcard = String::from("BEGIN:VCARD\r\nVERSION:3.0\r\n");

        vcard.push_str(&format!("UID:{}\r\n", contact.uid));
        vcard.push_str(&format!("FN:{}\r\n", contact.fn_));

        if let Some(ref ln) = contact.ln {
            // N field: Family;Given
            vcard.push_str(&format!("N:{};{}\r\n", ln, contact.fn_));
        } else {
            vcard.push_str(&format!("N:;{}\r\n", contact.fn_));
        }

        if let Some(ref email) = contact.email {
            vcard.push_str(&format!("EMAIL:{}\r\n", email));
        }

        if let Some(ref phone) = contact.phone {
            vcard.push_str(&format!("TEL:{}\r\n", phone));
        }

        if let Some(ref org) = contact.organization {
            vcard.push_str(&format!("ORG:{}\r\n", org));
        }

        if let Some(ref title) = contact.title {
            vcard.push_str(&format!("TITLE:{}\r\n", title));
        }

        if let Some(ref nickname) = contact.nickname {
            vcard.push_str(&format!("NICKNAME:{}\r\n", nickname));
        }

        if let Some(ref notes) = contact.notes {
            vcard.push_str(&format!("NOTE:{}\r\n", notes));
        }

        if let Some(ref birthday) = contact.birthday {
            vcard.push_str(&format!("BDAY:{}\r\n", birthday));
        }

        if let Some(ref url) = contact.url {
            vcard.push_str(&format!("URL:{}\r\n", url));
        }

        vcard.push_str("END:VCARD\r\n");

        Ok(vcard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_and_parse_contact() {
        let contact = Contact {
            uid: "contact-123".to_string(),
            fn_: "John".to_string(),
            ln: Some("Doe".to_string()),
            email: Some("john.doe@example.com".to_string()),
            phone: Some("+1234567890".to_string()),
            organization: Some("Acme Corp".to_string()),
            title: Some("Engineer".to_string()),
            nickname: Some("Johnny".to_string()),
            notes: Some("Test contact".to_string()),
            birthday: Some("1990-01-15".to_string()),
            url: Some("https://example.com".to_string()),
        };

        let vcard = CardDavClient::serialize_vcard(&contact).unwrap();

        // Verify vCard contains all fields
        assert!(vcard.contains("UID:contact-123"));
        assert!(vcard.contains("FN:John"));
        assert!(vcard.contains("N:Doe;John"));
        assert!(vcard.contains("EMAIL:john.doe@example.com"));
        assert!(vcard.contains("TEL:+1234567890"));
        assert!(vcard.contains("ORG:Acme Corp"));
        assert!(vcard.contains("TITLE:Engineer"));
        assert!(vcard.contains("NICKNAME:Johnny"));
        assert!(vcard.contains("NOTE:Test contact"));
        assert!(vcard.contains("BDAY:1990-01-15"));
        assert!(vcard.contains("URL:https://example.com"));

        // Parse it back
        let parsed = CardDavClient::parse_vcard(vcard.as_bytes()).unwrap();

        assert_eq!(parsed.uid, "contact-123");
        assert_eq!(parsed.fn_, "John");
        assert_eq!(parsed.ln, Some("Doe".to_string()));
        assert_eq!(parsed.email, Some("john.doe@example.com".to_string()));
        assert_eq!(parsed.phone, Some("+1234567890".to_string()));
        assert_eq!(parsed.organization, Some("Acme Corp".to_string()));
        assert_eq!(parsed.title, Some("Engineer".to_string()));
        assert_eq!(parsed.nickname, Some("Johnny".to_string()));
        assert_eq!(parsed.notes, Some("Test contact".to_string()));
        assert_eq!(parsed.birthday, Some("1990-01-15".to_string()));
        assert_eq!(parsed.url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_parse_vcard_minimal() {
        // Test with minimal required fields
        let vcard = b"BEGIN:VCARD
VERSION:3.0
UID:minimal-contact
FN:Jane Doe
END:VCARD";

        let contact = CardDavClient::parse_vcard(vcard).unwrap();

        assert_eq!(contact.uid, "minimal-contact");
        assert_eq!(contact.fn_, "Jane Doe");
        assert!(contact.ln.is_none());
        assert!(contact.email.is_none());
        assert!(contact.phone.is_none());
        assert!(contact.organization.is_none());
        assert!(contact.title.is_none());
        assert!(contact.nickname.is_none());
        assert!(contact.notes.is_none());
        assert!(contact.birthday.is_none());
        assert!(contact.url.is_none());
    }

    #[test]
    fn test_serialize_vcard_minimal() {
        let contact = Contact {
            uid: "minimal".to_string(),
            fn_: "Jane Doe".to_string(),
            ln: None,
            email: None,
            phone: None,
            organization: None,
            title: None,
            nickname: None,
            notes: None,
            birthday: None,
            url: None,
        };

        let vcard = CardDavClient::serialize_vcard(&contact).unwrap();

        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("VERSION:3.0"));
        assert!(vcard.contains("UID:minimal"));
        assert!(vcard.contains("FN:Jane Doe"));
        assert!(vcard.contains("END:VCARD"));
    }
}
