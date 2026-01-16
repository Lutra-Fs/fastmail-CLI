# WebDAV, CalDAV, and CardDAV Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement WebDAV, CalDAV (RFC 4791), and CardDAV (RFC 6352) protocol support for file storage, calendar management, and contact management via Fastmail's DAV endpoints using the `libdav` crate.

**Architecture:** Use the `libdav` crate for DAV protocol implementation, add CalDAV and CardDAV wrapper modules to `fastmail-client`, and add CLI commands for contacts, calendars, and files operations.

**Tech Stack:** Rust, libdav (DAV protocols), serde, serde_json, chrono, thiserror (errors), clap (CLI), tokio (async)

---

## Overview

This plan adds support for three DAV protocols using `libdav`:

1. **WebDAV (RFC 4918)** - Base protocol for file management
   - Use case: File storage and management via Fastmail Files

2. **CalDAV (RFC 4791)** - Calendar access extension
   - Use case: Calendar event management

3. **CardDAV (RFC 6352)** - Contacts access extension
   - Use case: Contact management

**Why `libdav`:**
- Mature, well-tested implementation of WebDAV/CalDAV/CardDAV
- Handles complex XML parsing and protocol edge cases
- Supports proper authentication, locks, and multi-status responses
- Reduces implementation complexity significantly

**Crate Structure:**
- `fastmail-client/dav.rs` - DAV client wrapper using libdav
- `fastmail-client/caldav.rs` - CalDAV-specific helpers
- `fastmail-client/carddav.rs` - CardDAV-specific helpers
- `fastmail-cli/commands/contacts.rs` - Contact CLI commands
- `fastmail-cli/commands/calendar.rs` - Calendar CLI commands
- `fastmail-cli/commands/files.rs` - File management CLI commands

---

## Task 1: Add libdav Dependency to fastmail-client

**Files:**
- Modify: `fastmail-client/Cargo.toml`

**Step 1: Add libdav and related dependencies**

```toml
# fastmail-client/Cargo.toml
[dependencies]
anyhow = { workspace = true }
chrono = "0.4"
directories = "5.0"
jmap-client = { path = "../jmap-client" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = "1.0"
toml = "0.8"

# DAV support
libdav = "0.10"
http = "1.0"
```

**Step 2: Run cargo check to verify dependencies**

Run: `cargo check -p fastmail-client`
Expected: OK (dependencies resolve)

**Step 3: Commit**

```bash
git add fastmail-client/Cargo.toml
git commit -m "feat(dav): add libdav dependency for WebDAV/CalDAV/CardDAV support"
```

---

## Task 2: Create DAV Client Wrapper

**Files:**
- Create: `fastmail-client/src/dav.rs`
- Modify: `fastmail-client/src/lib.rs`

**Step 1: Write DAV client wrapper module**

```rust
// fastmail-client/src/dav.rs
use crate::config::FastmailConfig;
use anyhow::Result;
use http::StatusCode;
use libdav::auth::Password;
use libdav::dav::DavClient;
use libdav::xml::Item as DavItem;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Generic DAV resource metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavResource {
    pub href: String,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub is_collection: bool,
}

impl From<DavItem> for DavResource {
    fn from(item: DavItem) -> Self {
        Self {
            href: item.href.to_string(),
            content_type: item.content_type,
            etag: item.etag,
            is_collection: item.is_collection(),
        }
    }
}

/// DAV client wrapper for Fastmail
pub struct DavClient {
    client: DavClient,
    base_url: String,
}

impl DavClient {
    /// Create a new DAV client from Fastmail config
    pub async fn from_config(config: &FastmailConfig, service: DavService) -> Result<Self> {
        let base_url = match service {
            DavService::Calendars => config.get_caldav_url(),
            DavService::AddressBooks => config.get_carddav_url(),
            DavService::Files => config.get_webdav_url(),
        };

        let account_id = config.account_id.clone().unwrap_or_else(|| "default".to_string());

        // Build full service URL
        let service_url = match service {
            DavService::Calendars => format!("{}/dav/calendars/user/{}/", base_url, account_id),
            DavService::AddressBooks => format!("{}/dav/addressbooks/user/{}/", base_url, account_id),
            DavService::Files => format!("{}/files/{}/", base_url, account_id),
        };

        // Create libdav client
        let password = Password::from(config.token.clone());
        let uri = service_url.parse()?;

        let client = DavClient::builder()
            .with_uri(uri)
            .with_password(password)
            .build()
            .await?;

        Ok(Self {
            client,
            base_url: service_url,
        })
    }

    /// Get the underlying libdav client
    pub fn inner(&self) -> &DavClient {
        &self.client
    }

    /// List resources at a given path
    pub async fn list(&self, path: &str, depth: u8) -> Result<Vec<DavResource>> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        let items = self.client
            .list_resources(&href, depth.into())
            .await?;

        Ok(items.into_iter().map(DavResource::from).collect())
    }

    /// Get properties for a single resource
    pub async fn get_properties(&self, path: &str) -> Result<DavResource> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        let items = self.client
            .list_resources(&href, libdav::dav::Depth::Zero)
            .await?;

        items
            .into_iter()
            .map(DavResource::from)
            .next()
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", path))
    }

    /// Create a collection
    pub async fn create_collection(&self, path: &str) -> Result<()> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        match self.client.create_collection(&href).await {
            Ok(_) => Ok(()),
            Err(e) if e.status() == Some(StatusCode::CONFLICT) => {
                Err(anyhow::anyhow!("Parent collection does not exist"))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Delete a resource
    pub async fn delete(&self, path: &str) -> Result<()> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        self.client.delete(&href).await?;
        Ok(())
    }

    /// Upload/put a resource
    pub async fn put(&self, path: &str, content: &[u8], content_type: &str) -> Result<String> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        let etag = self.client
            .put(&href, content, content_type.parse()?)
            .await?;

        Ok(etag.map(|e| e.to_string()).unwrap_or_default())
    }

    /// Get resource content
    pub async fn get(&self, path: &str) -> Result<Vec<u8>> {
        let href = self.build_href(path)?;

        use libdav::dav::DavClient as _;

        let (content, _) = self.client.get(&href).await?;
        Ok(content.to_vec())
    }

    /// Copy a resource
    pub async fn copy(&self, from: &str, to: &str, overwrite: bool) -> Result<()> {
        let from_href = self.build_href(from)?;
        let to_href = self.build_href(to)?;

        use libdav::dav::DavClient as _;

        self.client
            .copy(&from_href, &to_href, overwrite)
            .await?;
        Ok(())
    }

    /// Move a resource
    pub async fn move_resource(&self, from: &str, to: &str, overwrite: bool) -> Result<()> {
        let from_href = self.build_href(from)?;
        let to_href = self.build_href(to)?;

        use libdav::dav::DavClient as _;

        self.client
            .move_resource(&from_href, &to_href, overwrite)
            .await?;
        Ok(())
    }

    fn build_href(&self, path: &str) -> Result<http::Uri> {
        let path = path.trim_start_matches('/');

        let full = if path.is_empty() {
            self.base_url.clone()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        };

        Ok(full.parse()?)
    }
}

/// DAV service type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavService {
    Calendars,
    AddressBooks,
    Files,
}

// Convert u8 depth to libdav depth
impl From<u8> for libdav::dav::Depth {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Zero,
            1 => Self::One,
            _ => Self::Infinity,
        }
    }
}
```

**Step 2: Update lib.rs**

```rust
// fastmail-client/src/lib.rs
pub mod caldav;
pub mod carddav;
pub mod client;
pub mod config;
pub mod dav;
pub mod masked_email;
pub mod whitelist;

pub use dav::{DavClient, DavResource, DavService};
pub use client::FastmailClient;
pub use config::FastmailConfig;
```

**Step 3: Run cargo check**

Run: `cargo check -p fastmail-client`
Expected: OK (DAV client wrapper compiles)

**Step 4: Commit**

```bash
git add fastmail-client/src/dav.rs fastmail-client/src/lib.rs
git commit -m "feat(dav): add DavClient wrapper using libdav"
```

---

## Task 3: Create CalDAV Module

**Files:**
- Create: `fastmail-client/src/caldav.rs`

**Step 1: Write CalDAV module**

```rust
// fastmail-client/src/caldav.rs
use crate::config::FastmailConfig;
use crate::dav::{DavClient, DavService, DavResource};
use anyhow::Result;
use chrono::{DateTime, Utc};
use libdav::cal::CalDavClient;
use libdav::xml::Item as DavItem;
use serde::{Deserialize, Serialize};

/// Calendar event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub status: Option<String>,
}

/// Calendar information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub href: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
}

/// CalDAV client for Fastmail
pub struct CalDavClient {
    dav: DavClient,
    caldav: CalDavClient,
}

impl CalDavClient {
    /// Create a new CalDAV client from Fastmail config
    pub async fn from_config(config: &FastmailConfig) -> Result<Self> {
        let dav = DavClient::from_config(config, DavService::Calendars).await?;

        // Get the underlying libdav client
        let inner_client = dav.inner().clone();

        let caldav = CalDavClient::new(inner_client);

        Ok(Self { dav, caldav })
    }

    /// List all calendars
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>> {
        use libdav::cal::CalDavClient as _;

        let homeset = self.caldav
            .calendar_home_set()
            .await?
            .unwrap_or_else(|| "/".to_string());

        let resources = self.dav.list(&homeset, 1).await?;

        let calendars = resources
            .into_iter()
            .filter(|r| r.is_collection)
            .map(|r| Calendar {
                href: r.href,
                display_name: None,
                description: None,
                color: None,
            })
            .collect();

        Ok(calendars)
    }

    /// Get a specific calendar
    pub async fn get_calendar(&self, href: &str) -> Result<Calendar> {
        let resource = self.dav.get_properties(href).await?;

        Ok(Calendar {
            href: resource.href,
            display_name: None,
            description: None,
            color: None,
        })
    }

    /// Create a new calendar
    pub async fn create_calendar(&self, name: &str, description: Option<String>) -> Result<Calendar> {
        let path = format!("/{}", name);

        self.dav.create_collection(&path).await?;

        Ok(Calendar {
            href: path,
            display_name: Some(name.to_string()),
            description,
            color: None,
        })
    }

    /// Delete a calendar
    pub async fn delete_calendar(&self, href: &str) -> Result<()> {
        self.dav.delete(href).await
    }

    /// List events in a calendar
    pub async fn list_events(&self, calendar_href: &str) -> Result<Vec<CalendarEvent>> {
        use libdav::cal::CalDavClient as _;

        let resources = self.dav.list(calendar_href, 1).await?;

        let mut events = Vec::new();

        for resource in resources {
            // Skip calendar itself and non-icalendar resources
            if resource.is_collection {
                continue;
            }

            // Download and parse iCalendar data
            let ical_data = self.dav.get(&resource.href).await?;

            // Parse VEVENT from iCalendar
            if let Some(event) = Self::parse_icalendar_event(&ical_data) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get a specific event
    pub async fn get_event(&self, event_href: &str) -> Result<CalendarEvent> {
        let ical_data = self.dav.get(event_href).await?;

        Self::parse_icalendar_event(&ical_data)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse event"))
    }

    /// Create or update an event
    pub async fn put_event(&self, calendar_href: &str, event: &CalendarEvent) -> Result<String> {
        let filename = format!("{}.ics", event.uid);
        let path = if calendar_href.ends_with('/') {
            format!("{}{}", calendar_href, filename)
        } else {
            format!("{}/{}", calendar_href, filename)
        };

        let ical = Self::serialize_icalendar_event(event)?;

        self.dav.put(&path, ical.as_bytes(), "text/calendar;charset=utf-8").await?;

        Ok(path)
    }

    /// Delete an event
    pub async fn delete_event(&self, event_href: &str) -> Result<()> {
        self.dav.delete(event_href).await
    }

    /// Parse iCalendar VEVENT (simplified - MVP)
    fn parse_icalendar_event(data: &[u8]) -> Option<CalendarEvent> {
        let text = std::str::from_utf8(data).ok()?;
        let lines: Vec<&str> = text.lines().collect();

        let mut uid = None;
        let mut summary = None;
        let mut description = None;
        let mut start = None;
        let mut end = None;
        let mut location = None;
        let mut status = None;

        for line in lines {
            let line = line.trim();
            if line.starts_with("UID:") {
                uid = Some(line[4..].to_string());
            } else if line.starts_with("SUMMARY:") {
                summary = Some(line[8..].to_string());
            } else if line.starts_with("DESCRIPTION:") {
                description = Some(line[12..].to_string());
            } else if line.starts_with("DTSTART:") {
                if let Ok(dt) = Self::parse_ical_datetime(&line[8..]) {
                    start = Some(dt);
                }
            } else if line.starts_with("DTEND:") {
                if let Ok(dt) = Self::parse_ical_datetime(&line[6..]) {
                    end = Some(dt);
                }
            } else if line.starts_with("LOCATION:") {
                location = Some(line[9..].to_string());
            } else if line.starts_with("STATUS:") {
                status = Some(line[7..].to_string());
            }
        }

        Some(CalendarEvent {
            uid: uid?,
            summary,
            description,
            start: start?,
            end,
            location,
            status,
        })
    }

    /// Parse iCalendar datetime (simplified)
    fn parse_ical_datetime(s: &str) -> Result<DateTime<Utc>> {
        let s = s.trim().trim_end_matches('Z');

        if s.len() >= 15 {
            let year = s[0..4].parse::<i32>()?;
            let month = s[4..6].parse::<u32>()?;
            let day = s[6..8].parse::<u32>()?;
            let hour = s[9..11].parse::<u32>()?;
            let min = s[11..13].parse::<u32>()?;
            let sec = s[13..15].parse::<u32>()?;

            Ok(DateTime::from_naive_utc_and_offset(
                chrono::NaiveDateTime::new(
                    chrono::NaiveDate::from_ymd_opt(year, month, day)
                        .ok_or_else(|| anyhow::anyhow!("invalid date"))?,
                    chrono::NaiveTime::from_hms_opt(hour, min, sec)
                        .ok_or_else(|| anyhow::anyhow!("invalid time"))?,
                ),
                Utc,
            ))
        } else {
            anyhow::bail!("Invalid datetime format")
        }
    }

    /// Serialize event to iCalendar format (simplified)
    fn serialize_icalendar_event(event: &CalendarEvent) -> Result<String> {
        let mut ical = String::from("BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Fastmail CLI//EN\nBEGIN:VEVENT\n");

        ical.push_str(&format!("UID:{}\n", event.uid));
        ical.push_str(&format!("DTSTART:{}Z\n", event.start.format("%Y%m%dT%H%M%S")));

        if let Some(end) = &event.end {
            ical.push_str(&format!("DTEND:{}Z\n", end.format("%Y%m%dT%H%M%S")));
        }

        if let Some(summary) = &event.summary {
            ical.push_str(&format!("SUMMARY:{}\n", summary));
        }

        if let Some(description) = &event.description {
            ical.push_str(&format!("DESCRIPTION:{}\n", description));
        }

        if let Some(location) = &event.location {
            ical.push_str(&format!("LOCATION:{}\n", location));
        }

        if let Some(status) = &event.status {
            ical.push_str(&format!("STATUS:{}\n", status));
        }

        ical.push_str("END:VEVENT\nEND:VCALENDAR\n");

        Ok(ical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ical_datetime() {
        let result = CalDavClient::parse_ical_datetime("20240115T120000Z");
        assert!(result.is_ok());
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fastmail-client`
Expected: OK (CalDAV module compiles)

**Step 3: Update lib.rs to export CalDav types**

```rust
// fastmail-client/src/lib.rs
pub mod caldav;
pub mod carddav;
pub mod client;
pub mod config;
pub mod dav;
pub mod masked_email;
pub mod whitelist;

pub use caldav::{CalDavClient, Calendar, CalendarEvent};
pub use carddav::{CardDavClient, Contact, AddressBook};
pub use dav::{DavClient, DavResource, DavService};
pub use client::FastmailClient;
pub use config::FastmailConfig;
```

**Step 4: Run cargo check again**

Run: `cargo check -p fastmail-client`
Expected: OK (everything compiles)

**Step 5: Commit**

```bash
git add fastmail-client/src/caldav.rs fastmail-client/src/lib.rs
git commit -m "feat(caldav): add CalDavClient using libdav"
```

---

## Task 4: Create CardDAV Module

**Files:**
- Create: `fastmail-client/src/carddav.rs`

**Step 1: Write CardDAV module**

```rust
// fastmail-client/src/carddav.rs
use crate::config::FastmailConfig;
use crate::dav::{DavClient, DavService, DavResource};
use anyhow::Result;
use libdav::card::CardDavClient;
use serde::{Deserialize, Serialize};

/// vCard contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub uid: String,
    pub fn_: Option<String>,
    pub ln: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub nickname: Option<String>,
    pub notes: Option<String>,
    pub birthday: Option<String>,
    pub url: Option<String>,
}

/// Address book information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    pub href: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

/// CardDAV client for Fastmail
pub struct CardDavClient {
    dav: DavClient,
    carddav: CardDavClient,
}

impl CardDavClient {
    /// Create a new CardDAV client from Fastmail config
    pub async fn from_config(config: &FastmailConfig) -> Result<Self> {
        let dav = DavClient::from_config(config, DavService::AddressBooks).await?;

        let inner_client = dav.inner().clone();

        let carddav = CardDavClient::new(inner_client);

        Ok(Self { dav, carddav })
    }

    /// List all address books
    pub async fn list_address_books(&self) -> Result<Vec<AddressBook>> {
        use libdav::card::CardDavClient as _;

        let homeset = self.carddav
            .address_book_home_set()
            .await?
            .unwrap_or_else(|| "/".to_string());

        let resources = self.dav.list(&homeset, 1).await?;

        let books = resources
            .into_iter()
            .filter(|r| r.is_collection)
            .map(|r| AddressBook {
                href: r.href,
                display_name: None,
                description: None,
            })
            .collect();

        Ok(books)
    }

    /// Get a specific address book
    pub async fn get_address_book(&self, href: &str) -> Result<AddressBook> {
        let resource = self.dav.get_properties(href).await?;

        Ok(AddressBook {
            href: resource.href,
            display_name: None,
            description: None,
        })
    }

    /// Create a new address book
    pub async fn create_address_book(&self, name: &str, description: Option<String>) -> Result<AddressBook> {
        let path = format!("/{}", name);

        self.dav.create_collection(&path).await?;

        Ok(AddressBook {
            href: path,
            display_name: Some(name.to_string()),
            description,
        })
    }

    /// Delete an address book
    pub async fn delete_address_book(&self, href: &str) -> Result<()> {
        self.dav.delete(href).await
    }

    /// List contacts in an address book
    pub async fn list_contacts(&self, addressbook_href: &str) -> Result<Vec<Contact>> {
        let resources = self.dav.list(addressbook_href, 1).await?;

        let mut contacts = Vec::new();

        for resource in resources {
            if resource.is_collection {
                continue;
            }

            let vcard_data = self.dav.get(&resource.href).await?;

            if let Some(contact) = Self::parse_vcard(&vcard_data) {
                contacts.push(contact);
            }
        }

        Ok(contacts)
    }

    /// Get a specific contact
    pub async fn get_contact(&self, contact_href: &str) -> Result<Contact> {
        let vcard_data = self.dav.get(contact_href).await?;

        Self::parse_vcard(&vcard_data)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse contact"))
    }

    /// Create or update a contact
    pub async fn put_contact(&self, addressbook_href: &str, contact: &Contact) -> Result<String> {
        let filename = format!("{}.vcf", contact.uid);
        let path = if addressbook_href.ends_with('/') {
            format!("{}{}", addressbook_href, filename)
        } else {
            format!("{}/{}", addressbook_href, filename)
        };

        let vcard = Self::serialize_vcard(contact)?;

        self.dav.put(&path, vcard.as_bytes(), "text/vcard;charset=utf-8").await?;

        Ok(path)
    }

    /// Delete a contact
    pub async fn delete_contact(&self, contact_href: &str) -> Result<()> {
        self.dav.delete(contact_href).await
    }

    /// Search contacts by email or name
    pub async fn search_contacts(&self, addressbook_href: &str, query: &str) -> Result<Vec<Contact>> {
        let contacts = self.list_contacts(addressbook_href).await?;

        let query_lower = query.to_lowercase();

        Ok(contacts
            .into_iter()
            .filter(|c| {
                c.email.as_ref().map_or(false, |e| e.to_lowercase().contains(&query_lower))
                    || c.fn_.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower))
                    || c.ln.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower))
                    || c.nickname.as_ref().map_or(false, |n| n.to_lowercase().contains(&query_lower))
            })
            .collect())
    }

    /// Parse vCard (simplified vCard 3.0 parsing)
    fn parse_vcard(data: &[u8]) -> Option<Contact> {
        let text = std::str::from_utf8(data).ok()?;
        let lines: Vec<&str> = text.lines().collect();

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

        for line in lines {
            let line = line.trim();
            if line.starts_with("UID:") {
                uid = Some(line[4..].to_string());
            } else if line.starts_with("FN:") {
                fn_ = Some(line[3..].to_string());
            } else if line.starts_with("N:") {
                let parts: Vec<&str> = line[2..].split(';').collect();
                if !parts.is_empty() && !parts[0].is_empty() {
                    ln = Some(parts[0].to_string());
                }
            } else if line.starts_with("EMAIL") {
                if let Some(pos) = line.find(':') {
                    email = Some(line[pos + 1..].to_string());
                }
            } else if line.starts_with("TEL") {
                if let Some(pos) = line.find(':') {
                    phone = Some(line[pos + 1..].to_string());
                }
            } else if line.starts_with("ORG:") {
                organization = Some(line[4..].to_string());
            } else if line.starts_with("TITLE:") {
                title = Some(line[6..].to_string());
            } else if line.starts_with("NICKNAME:") {
                nickname = Some(line[9..].to_string());
            } else if line.starts_with("NOTE:") {
                notes = Some(line[5..].to_string());
            } else if line.starts_with("BDAY:") {
                birthday = Some(line[5..].to_string());
            } else if line.starts_with("URL:") {
                url = Some(line[4..].to_string());
            }
        }

        Some(Contact {
            uid: uid?,
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

    /// Serialize contact to vCard format (simplified vCard 3.0)
    fn serialize_vcard(contact: &Contact) -> Result<String> {
        let mut vcard = String::from("BEGIN:VCARD\nVERSION:3.0\n");

        vcard.push_str(&format!("UID:{}\n", contact.uid));
        vcard.push_str(&format!("PRODID:-//Fastmail CLI//EN\n"));

        if let Some(fn_) = &contact.fn_ {
            vcard.push_str(&format!("FN:{}\n", fn_));
        }

        if let Some(ln) = &contact.ln {
            vcard.push_str(&format!("N:{};;;\n", ln));
        }

        if let Some(email) = &contact.email {
            vcard.push_str(&format!("EMAIL;TYPE=INTERNET:{}\n", email));
        }

        if let Some(phone) = &contact.phone {
            vcard.push_str(&format!("TEL;TYPE=CELL:{}\n", phone));
        }

        if let Some(org) = &contact.organization {
            vcard.push_str(&format!("ORG:{}\n", org));
        }

        if let Some(title) = &contact.title {
            vcard.push_str(&format!("TITLE:{}\n", title));
        }

        if let Some(nickname) = &contact.nickname {
            vcard.push_str(&format!("NICKNAME:{}\n", nickname));
        }

        if let Some(notes) = &contact.notes {
            vcard.push_str(&format!("NOTE:{}\n", notes));
        }

        if let Some(birthday) = &contact.birthday {
            vcard.push_str(&format!("BDAY:{}\n", birthday));
        }

        if let Some(url) = &contact.url {
            vcard.push_str(&format!("URL:{}\n", url));
        }

        vcard.push_str("END:VCARD\n");

        Ok(vcard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_and_parse_contact() {
        let contact = Contact {
            uid: "test-123".to_string(),
            fn_: Some("John Doe".to_string()),
            ln: Some("Doe".to_string()),
            email: Some("john@example.com".to_string()),
            phone: Some("+1234567890".to_string()),
            organization: Some("Acme Inc".to_string()),
            title: Some("Engineer".to_string()),
            nickname: Some("Johnny".to_string()),
            notes: Some("Test contact".to_string()),
            birthday: Some("1990-01-15".to_string()),
            url: Some("https://example.com".to_string()),
        };

        let vcard = CardDavClient::serialize_vcard(&contact).unwrap();
        let parsed = CardDavClient::parse_vcard(vcard.as_bytes()).unwrap();

        assert_eq!(parsed.uid, contact.uid);
        assert_eq!(parsed.fn_, contact.fn_);
        assert_eq!(parsed.email, contact.email);
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fastmail-client`
Expected: OK (CardDAV module compiles)

**Step 3: Commit**

```bash
git add fastmail-client/src/carddav.rs
git commit -m "feat(carddav): add CardDavClient using libdav"
```

---

## Task 5: Update Config for DAV Endpoints

**Files:**
- Modify: `fastmail-client/src/config.rs`

**Step 1: Add DAV endpoint configuration**

```rust
// fastmail-client/src/config.rs
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastmailConfig {
    pub account_id: Option<String>,
    pub token: String,
    #[serde(default)]
    pub dav_endpoints: Option<DavEndpoints>,
    #[serde(default)]
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavEndpoints {
    #[serde(default = "default_caldav_url")]
    pub caldav: String,
    #[serde(default = "default_carddav_url")]
    pub carddav: String,
    #[serde(default = "default_webdav_url")]
    pub webdav: String,
}

fn default_caldav_url() -> String {
    "https://dav.fastmail.com".to_string()
}

fn default_carddav_url() -> String {
    "https://dav.fastmail.com".to_string()
}

fn default_webdav_url() -> String {
    "https://dav.fastmail.com".to_string()
}

impl Default for DavEndpoints {
    fn default() -> Self {
        Self {
            caldav: default_caldav_url(),
            carddav: default_carddav_url(),
            webdav: default_webdav_url(),
        }
    }
}

impl FastmailConfig {
    pub fn load() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("com", "fastmail", "fastmail-cli")
            .ok_or_else(|| anyhow::anyhow!("Failed to determine config directory"))?;

        let config_path = config_dir.config_dir().join("config.toml");

        if !config_path.exists() {
            return Ok(Self {
                account_id: None,
                token: std::env::var("FASTMAIL_TOKEN").unwrap_or_default(),
                dav_endpoints: Some(DavEndpoints::default()),
                whitelist: vec![],
            });
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: FastmailConfig = toml::from_str(&content)?;

        Ok(config)
    }

    pub fn get_caldav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.caldav.clone())
            .unwrap_or_else(default_caldav_url)
    }

    pub fn get_carddav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.carddav.clone())
            .unwrap_or_else(default_carddav_url)
    }

    pub fn get_webdav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.webdav.clone())
            .unwrap_or_else(default_webdav_url)
    }
}
```

**Step 2: Run cargo check**

Run: `cargo check -p fastmail-client`
Expected: OK (config updates compile)

**Step 3: Commit**

```bash
git add fastmail-client/src/config.rs
git commit -m "feat(config): add DAV endpoint configuration"
```

---

## Task 6: Add Contacts CLI Commands

**Files:**
- Create: `fastmail-cli/src/commands/contacts.rs`
- Modify: `fastmail-cli/src/commands/mod.rs`
- Modify: `fastmail-cli/src/main.rs`
- Modify: `fastmail-cli/Cargo.toml`

**Step 1: Add chrono dependency to CLI**

```toml
# fastmail-cli/Cargo.toml
[dependencies]
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.5", features = ["derive"] }
fastmail-client = { path = "../fastmail-client" }
```

**Step 2: Write contacts command module**

```rust
// fastmail-cli/src/commands/contacts.rs
use crate::output::{Response, Meta};
use anyhow::Result;
use clap::{Parser, Subcommand};
use fastmail_client::{CardDavClient, Contact};

#[derive(Subcommand, Clone, Debug)]
pub enum ContactsCommands {
    /// List all address books
    ListBooks {
        /// Filter address books by name
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get a specific address book
    GetBook {
        /// Address book href/path
        href: String,
    },
    /// Create a new address book
    CreateBook {
        /// Address book name
        name: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete an address book
    DeleteBook {
        /// Address book href/path
        href: String,
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// List contacts
    List {
        /// Address book href (defaults to first)
        #[arg(short, long)]
        book: Option<String>,
        /// Search query
        #[arg(short, long)]
        search: Option<String>,
        /// Max results
        #[arg(short, long, default = "100")]
        limit: usize,
    },
    /// Get a specific contact
    Get {
        /// Contact href/path
        href: String,
    },
    /// Create a contact (JSON input)
    Create {
        /// Address book href
        #[arg(short, long)]
        book: String,
        /// Contact data as JSON
        #[arg(short, long)]
        data: String,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a contact
    Delete {
        /// Contact href/path
        href: String,
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_contacts(cmd: ContactsCommands) -> Result<Response serde_json::Value>> {
    let config = fastmail_client::FastmailConfig::load()?;
    let client = CardDavClient::from_config(&config).await?;

    match cmd {
        ContactsCommands::ListBooks { filter } => {
            let books = client.list_address_books().await?;

            let filtered: Vec<_> = if let Some(f) = filter {
                books.into_iter()
                    .filter(|b| b.display_name.as_ref().map_or(false, |n| n.contains(&f)))
                    .collect()
            } else {
                books
            };

            Ok(Response::ok_with_meta(
                serde_json::to_value(filtered)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::GetBook { href } => {
            let book = client.get_address_book(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(book)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::CreateBook { name, description, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "name": name, "description": description }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let book = client.create_address_book(&name, description).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(book)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::DeleteBook { href, force, dry_run } => {
            if !force && !dry_run {
                println!("Delete address book '{}'? [y/N]", href);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(Response::ok_with_meta(
                        serde_json::json!({ "cancelled": true }),
                        Meta { dry_run: Some(true), ..Default::default() },
                    ));
                }
            }

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "deleted": href, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.delete_address_book(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "deleted": href }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::List { book, search, limit } => {
            let book_href = if let Some(b) = book {
                b
            } else {
                let books = client.list_address_books().await?;
                books.first()
                    .ok_or_else(|| anyhow::anyhow!("No address books found"))?
                    .href
                    .clone()
            };

            let contacts = if let Some(query) = search {
                client.search_contacts(&book_href, &query).await?
            } else {
                client.list_contacts(&book_href).await?
            };

            let limited: Vec<_> = contacts.into_iter().take(limit).collect();

            Ok(Response::ok_with_meta(
                serde_json::to_value(limited)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::Get { href } => {
            let contact = client.get_contact(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(contact)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::Create { book, data, dry_run } => {
            let contact: Contact = serde_json::from_str(&data)?;

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::to_value(contact)?,
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let href = client.put_contact(&book, &contact).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "created": href, "uid": contact.uid }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        ContactsCommands::Delete { href, force, dry_run } => {
            if !force && !dry_run {
                println!("Delete contact '{}'? [y/N]", href);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(Response::ok_with_meta(
                        serde_json::json!({ "cancelled": true }),
                        Meta { dry_run: Some(true), ..Default::default() },
                    ));
                }
            }

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "deleted": href, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.delete_contact(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "deleted": href }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
    }
}
```

**Step 3: Update commands/mod.rs**

```rust
// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod contacts;
pub mod mail;
pub mod mailbox;
pub mod masked;

pub use contacts::{ContactsCommands, handle_contacts};
```

**Step 4: Update main.rs to add contacts command**

```rust
// fastmail-cli/src/main.rs
use clap::{Parser, Subcommand};
use commands::{handle_blob, handle_contacts, handle_mail, handle_mailbox, handle_masked};

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Blob(BlobCommands),
    Contacts(ContactsCommands),
    Mail(MailCommands),
    Mailbox(MailboxCommands),
    Masked(MaskedCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let response = match cli.command {
        Commands::Blob(cmd) => handle_blob(cmd).await?,
        Commands::Contacts(cmd) => handle_contacts(cmd).await?,
        Commands::Mail(cmd) => handle_mail(cmd).await?,
        Commands::Mailbox(cmd) => handle_mailbox(cmd).await?,
        Commands::Masked(cmd) => handle_masked(cmd).await?,
    };

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
```

**Step 5: Run cargo check**

Run: `cargo check`
Expected: OK (CLI compiles)

**Step 6: Commit**

```bash
git add fastmail-cli/Cargo.toml fastmail-cli/src/commands/contacts.rs fastmail-cli/src/commands/mod.rs fastmail-cli/src/main.rs
git commit -m "feat(cli): add contacts command"
```

---

## Task 7: Add Calendar CLI Commands

**Files:**
- Create: `fastmail-cli/src/commands/calendar.rs`
- Modify: `fastmail-cli/src/commands/mod.rs`
- Modify: `fastmail-cli/src/main.rs`

**Step 1: Write calendar command module**

```rust
// fastmail-cli/src/commands/calendar.rs
use crate::output::{Response, Meta};
use anyhow::Result;
use clap::{Parser, Subcommand};
use chrono::{DateTime, Utc};
use fastmail_client::{CalDavClient, CalendarEvent};

#[derive(Subcommand, Clone, Debug)]
pub enum CalendarCommands {
    /// List all calendars
    List {
        /// Filter by name
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get a specific calendar
    Get {
        /// Calendar href/path
        href: String,
    },
    /// Create a calendar
    Create {
        /// Calendar name
        name: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a calendar
    Delete {
        /// Calendar href/path
        href: String,
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// List events
    ListEvents {
        /// Calendar href (defaults to first)
        #[arg(short, long)]
        calendar: Option<String>,
        /// Start date (ISO 8601)
        #[arg(short, long)]
        from: Option<String>,
        /// End date (ISO 8601)
        #[arg(short, long)]
        to: Option<String>,
        /// Max results
        #[arg(short, long, default = "100")]
        limit: usize,
    },
    /// Get a specific event
    GetEvent {
        /// Event href/path
        href: String,
    },
    /// Create an event (JSON input)
    CreateEvent {
        /// Calendar href
        #[arg(short, long)]
        calendar: String,
        /// Event data as JSON
        #[arg(short, long)]
        data: String,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete an event
    DeleteEvent {
        /// Event href/path
        href: String,
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_calendar(cmd: CalendarCommands) -> Result<Response serde_json::Value>> {
    let config = fastmail_client::FastmailConfig::load()?;
    let client = CalDavClient::from_config(&config).await?;

    match cmd {
        CalendarCommands::List { filter } => {
            let calendars = client.list_calendars().await?;

            let filtered: Vec<_> = if let Some(f) = filter {
                calendars.into_iter()
                    .filter(|c| c.display_name.as_ref().map_or(false, |n| n.contains(&f)))
                    .collect()
            } else {
                calendars
            };

            Ok(Response::ok_with_meta(
                serde_json::to_value(filtered)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::Get { href } => {
            let calendar = client.get_calendar(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(calendar)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::Create { name, description, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "name": name, "description": description }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let calendar = client.create_calendar(&name, description).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(calendar)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::Delete { href, force, dry_run } => {
            if !force && !dry_run {
                println!("Delete calendar '{}'? [y/N]", href);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(Response::ok_with_meta(
                        serde_json::json!({ "cancelled": true }),
                        Meta { dry_run: Some(true), ..Default::default() },
                    ));
                }
            }

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "deleted": href, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.delete_calendar(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "deleted": href }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::ListEvents { calendar, from, to, limit } => {
            let cal_href = if let Some(c) = calendar {
                c
            } else {
                let calendars = client.list_calendars().await?;
                calendars.first()
                    .ok_or_else(|| anyhow::anyhow!("No calendars found"))?
                    .href
                    .clone()
            };

            let mut events = client.list_events(&cal_href).await?;

            if let Some(from_str) = from {
                if let Ok(from_dt) = DateTime::parse_from_rfc3339(&from_str) {
                    let from_utc = from_dt.with_timezone(&Utc);
                    events.retain(|e| e.start >= from_utc);
                }
            }

            if let Some(to_str) = to {
                if let Ok(to_dt) = DateTime::parse_from_rfc3339(&to_str) {
                    let to_utc = to_dt.with_timezone(&Utc);
                    events.retain(|e| e.start <= to_utc);
                }
            }

            let limited: Vec<_> = events.into_iter().take(limit).collect();

            Ok(Response::ok_with_meta(
                serde_json::to_value(limited)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::GetEvent { href } => {
            let event = client.get_event(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::to_value(event)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::CreateEvent { calendar, data, dry_run } => {
            let event: CalendarEvent = serde_json::from_str(&data)?;

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::to_value(event)?,
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let href = client.put_event(&calendar, &event).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "created": href, "uid": event.uid }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        CalendarCommands::DeleteEvent { href, force, dry_run } => {
            if !force && !dry_run {
                println!("Delete event '{}'? [y/N]", href);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(Response::ok_with_meta(
                        serde_json::json!({ "cancelled": true }),
                        Meta { dry_run: Some(true), ..Default::default() },
                    ));
                }
            }

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "deleted": href, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.delete_event(&href).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "deleted": href }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
    }
}
```

**Step 2: Update commands/mod.rs**

```rust
// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod calendar;
pub mod contacts;
pub mod mail;
pub mod mailbox;
pub mod masked;

pub use calendar::{CalendarCommands, handle_calendar};
pub use contacts::{ContactsCommands, handle_contacts};
```

**Step 3: Update main.rs**

```rust
// fastmail-cli/src/main.rs
use clap::{Parser, Subcommand};
use commands::{
    handle_blob, handle_calendar, handle_contacts, handle_mail, handle_mailbox, handle_masked
};

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Blob(BlobCommands),
    Calendar(CalendarCommands),
    Contacts(ContactsCommands),
    Mail(MailCommands),
    Mailbox(MailboxCommands),
    Masked(MaskedCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let response = match cli.command {
        Commands::Blob(cmd) => handle_blob(cmd).await?,
        Commands::Calendar(cmd) => handle_calendar(cmd).await?,
        Commands::Contacts(cmd) => handle_contacts(cmd).await?,
        Commands::Mail(cmd) => handle_mail(cmd).await?,
        Commands::Mailbox(cmd) => handle_mailbox(cmd).await?,
        Commands::Masked(cmd) => handle_masked(cmd).await?,
    };

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
```

**Step 4: Run cargo check**

Run: `cargo check`
Expected: OK

**Step 5: Commit**

```bash
git add fastmail-cli/src/commands/calendar.rs fastmail-cli/src/commands/mod.rs fastmail-cli/src/main.rs
git commit -m "feat(cli): add calendar command"
```

---

## Task 8: Add Files CLI Commands

**Files:**
- Create: `fastmail-cli/src/commands/files.rs`
- Modify: `fastmail-cli/src/commands/mod.rs`
- Modify: `fastmail-cli/src/main.rs`
- Modify: `fastmail-cli/Cargo.toml`

**Step 1: Add mime_guess dependency**

```toml
# fastmail-cli/Cargo.toml
[dependencies]
anyhow = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
mime-guess = "2.0"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.5", features = ["derive"] }
fastmail-client = { path = "../fastmail-client" }
```

**Step 2: Write files command module**

```rust
// fastmail-cli/src/commands/files.rs
use crate::output::{Response, Meta};
use anyhow::Result;
use clap::{Parser, Subcommand};
use fastmail_client::{FastmailConfig, DavClient, DavService};

#[derive(Subcommand, Clone, Debug)]
pub enum FilesCommands {
    /// List files
    List {
        /// Directory path
        #[arg(default = "/")]
        path: String,
        /// Depth (0=only, 1=children, infinity=recursive)
        #[arg(short, long, default = "1")]
        depth: u8,
        /// Filter by name
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get file info
    Info {
        /// File path
        path: String,
    },
    /// Upload a file
    Upload {
        /// Local file path
        local: String,
        /// Remote path
        remote: String,
        /// Content type
        #[arg(short, long)]
        content_type: Option<String>,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Download a file
    Download {
        /// Remote path
        remote: String,
        /// Local path
        local: String,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete
    Delete {
        /// Path
        path: String,
        /// Skip confirmation
        #[arg(long)]
        force: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Copy
    Copy {
        /// Source
        from: String,
        /// Destination
        to: String,
        /// Overwrite
        #[arg(long, default = "false")]
        overwrite: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Move
    Move {
        /// Source
        from: String,
        /// Destination
        to: String,
        /// Overwrite
        #[arg(long, default = "false")]
        overwrite: bool,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    /// Create directory
    Mkdir {
        /// Directory path
        path: String,
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_files(cmd: FilesCommands) -> Result<Response serde_json::Value>> {
    let config = FastmailConfig::load()?;
    let client = DavClient::from_config(&config, DavService::Files).await?;

    match cmd {
        FilesCommands::List { path, depth, filter } => {
            let resources = client.list(&path, depth).await?;

            let filtered: Vec<_> = if let Some(pattern) = filter {
                resources.into_iter()
                    .filter(|r| r.href.contains(&pattern))
                    .collect()
            } else {
                resources
            };

            let result: Vec<_> = filtered.into_iter()
                .map(|r| {
                    serde_json::json!({
                        "href": r.href,
                        "content_type": r.content_type,
                        "etag": r.etag,
                        "is_collection": r.is_collection,
                    })
                })
                .collect();

            Ok(Response::ok_with_meta(
                serde_json::to_value(result)?,
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Info { path } => {
            let resource = client.get_properties(&path).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({
                    "href": resource.href,
                    "content_type": resource.content_type,
                    "etag": resource.etag,
                    "is_collection": resource.is_collection,
                }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Upload { local, remote, content_type, dry_run } => {
            let content = std::fs::read(&local)?;

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({
                        "local": local,
                        "remote": remote,
                        "size": content.len(),
                    }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let ct = content_type.unwrap_or_else(|| {
                mime_guess::from_path(&local)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string())
            });

            let etag = client.put(&remote, &content, &ct).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({
                    "uploaded": remote,
                    "etag": etag,
                    "size": content.len(),
                }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Download { remote, local, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "remote": remote, "local": local }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            let content = client.get(&remote).await?;
            std::fs::write(&local, &content)?;

            Ok(Response::ok_with_meta(
                serde_json::json!({
                    "downloaded": remote,
                    "local": local,
                    "size": content.len(),
                }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Delete { path, force, dry_run } => {
            if !force && !dry_run {
                println!("Delete '{}'? [y/N]", path);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    return Ok(Response::ok_with_meta(
                        serde_json::json!({ "cancelled": true }),
                        Meta { dry_run: Some(true), ..Default::default() },
                    ));
                }
            }

            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "deleted": path, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.delete(&path).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "deleted": path }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Copy { from, to, overwrite, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "copy": { "from": from, "to": to } }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.copy(&from, &to, overwrite).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "copied": { "from": from, "to": to } }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Move { from, to, overwrite, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "move": { "from": from, "to": to } }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.move_resource(&from, &to, overwrite).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "moved": { "from": from, "to": to } }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
        FilesCommands::Mkdir { path, dry_run } => {
            if dry_run {
                return Ok(Response::ok_with_meta(
                    serde_json::json!({ "created": path, "dry_run": true }),
                    Meta { dry_run: Some(true), ..Default::default() },
                ));
            }

            client.create_collection(&path).await?;

            Ok(Response::ok_with_meta(
                serde_json::json!({ "created": path }),
                Meta {
                    dry_run: Some(false),
                    ..Default::default()
                },
            ))
        }
    }
}
```

**Step 3: Update commands/mod.rs**

```rust
// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod calendar;
pub mod contacts;
pub mod files;
pub mod mail;
pub mod mailbox;
pub mod masked;

pub use files::{FilesCommands, handle_files};
```

**Step 4: Update main.rs**

```rust
// fastmail-cli/src/main.rs
use clap::{Parser, Subcommand};
use commands::{
    handle_blob, handle_calendar, handle_contacts, handle_files,
    handle_mail, handle_mailbox, handle_masked
};

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    Blob(BlobCommands),
    Calendar(CalendarCommands),
    Contacts(ContactsCommands),
    Files(FilesCommands),
    Mail(MailCommands),
    Mailbox(MailboxCommands),
    Masked(MaskedCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let response = match cli.command {
        Commands::Blob(cmd) => handle_blob(cmd).await?,
        Commands::Calendar(cmd) => handle_calendar(cmd).await?,
        Commands::Contacts(cmd) => handle_contacts(cmd).await?,
        Commands::Files(cmd) => handle_files(cmd).await?,
        Commands::Mail(cmd) => handle_mail(cmd).await?,
        Commands::Mailbox(cmd) => handle_mailbox(cmd).await?,
        Commands::Masked(cmd) => handle_masked(cmd).await?,
    };

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
```

**Step 5: Run cargo check**

Run: `cargo check`
Expected: OK

**Step 6: Commit**

```bash
git add fastmail-cli/Cargo.toml fastmail-cli/src/commands/files.rs fastmail-cli/src/commands/mod.rs fastmail-cli/src/main.rs
git commit -m "feat(cli): add files command for WebDAV file management"
```

---

## Task 9: Final Verification and Documentation

**Step 1: Run full workspace check**

Run: `cargo check --workspace`
Expected: All crates compile

**Step 2: Run all tests**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 3: Build release binary**

Run: `cargo build --release`
Expected: Binary builds

**Step 4: Test CLI help**

Run: `./target/release/fastmail --help`
Expected: Shows all commands

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(dav): complete WebDAV/CalDAV/CardDAV implementation using libdav"
```

---

## Summary

This plan implements WebDAV/CalDAV/CardDAV support using the mature `libdav` crate instead of building from scratch, which:

- **Reduces complexity** - No need to implement XML parsing, HTTP edge cases, locks
- **Improves reliability** - Leverages battle-tested protocol implementation
- **Faster implementation** - ~9 tasks vs 13+ for from-scratch approach

### Key Files Created:
- `fastmail-client/src/dav.rs` - DAV client wrapper
- `fastmail-client/src/caldav.rs` - Calendar operations
- `fastmail-client/src/carddav.rs` - Contact operations
- `fastmail-cli/src/commands/{contacts,calendar,files}.rs` - CLI commands

### Dependencies Added:
- `libdav = "0.10"` - DAV protocol implementation
- `http = "1.0"` - HTTP types for libdav
- `chrono = "0.4"` - Datetime handling
- `mime-guess = "2.0"` - Content-type detection
