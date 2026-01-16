// fastmail-client/src/caldav.rs
//! CalDAV client implementation for calendar operations.

use crate::config::Config;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use libdav::caldav::{CreateCalendar, FindCalendars, GetCalendarResources, CalendarComponent};
use libdav::dav::{Delete, FoundCollection, PutResource, WebDavClient};
use libdav::FetchedResource;
use serde::{Deserialize, Serialize};
use tower_http::auth::AddAuthorization;

/// A calendar event (VEVENT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Unique identifier for the event
    pub uid: String,
    /// Event title/summary
    pub summary: String,
    /// Detailed description
    pub description: Option<String>,
    /// Event start time
    pub start: DateTime<Utc>,
    /// Event end time
    pub end: DateTime<Utc>,
    /// Event location
    pub location: Option<String>,
    /// Event status (e.g., "CONFIRMED", "TENTATIVE", "CANCELLED")
    pub status: Option<String>,
}

/// A calendar collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    /// Href (path) to the calendar
    pub href: String,
    /// Display name for the calendar
    pub display_name: Option<String>,
    /// Description of the calendar
    pub description: Option<String>,
    /// Color for the calendar (hex format)
    pub color: Option<String>,
}

impl From<FoundCollection> for Calendar {
    fn from(collection: FoundCollection) -> Self {
        Self {
            href: collection.href,
            display_name: None, // Would require additional PROPFIND
            description: None,
            color: None,
        }
    }
}

/// CalDAV client wrapper
///
/// Wraps the libdav CalDavClient with a simplified API specific to Fastmail.
pub struct CalDavClient {
    /// The underlying libdav CalDavClient (boxed to hide complex generics)
    caldav: Box<dyn CalDavClientInner>,
    /// Base URL for CalDAV operations
    base_url: String,
}

/// Trait to abstract over the complex CalDavClient generic type.
#[async_trait::async_trait]
trait CalDavClientInner: Send + Sync {
    async fn find_calendars(&self, home_set: &Uri) -> Result<Vec<FoundCollection>>;
    async fn get_calendar_resources(&self, href: &str) -> Result<Vec<FetchedResource>>;
    async fn delete_resource(&self, href: &str) -> Result<()>;
    async fn put_resource(&self, href: &str, data: String, content_type: &str) -> Result<Option<String>>;
    async fn create_calendar(&self, href: &str, display_name: &str) -> Result<()>;
}

/// Concrete implementation of CalDavClientInner
struct CalDavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = http::Response<hyper::body::Incoming>> + Send + Sync + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    client: libdav::CalDavClient<C>,
}

#[async_trait::async_trait]
impl<C> CalDavClientInner for CalDavClientInnerImpl<C>
where
    C: tower_service::Service<http::Request<String>, Response = http::Response<hyper::body::Incoming>>
        + Send
        + Sync
        + Clone
        + 'static,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>> + std::error::Error + Send + Sync,
    C::Future: Send + 'static,
{
    async fn find_calendars(&self, home_set: &Uri) -> Result<Vec<FoundCollection>> {
        let response = self.client.request(FindCalendars::new(home_set)).await?;
        Ok(response.calendars)
    }

    async fn get_calendar_resources(&self, href: &str) -> Result<Vec<FetchedResource>> {
        let response = self.client.request(GetCalendarResources::new(href)).await?;
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

    async fn create_calendar(&self, href: &str, display_name: &str) -> Result<()> {
        let create_calendar = CreateCalendar::new(href)
            .with_display_name(display_name)
            .with_components(&[CalendarComponent::VEvent]);
        self.client.request(create_calendar).await?;
        Ok(())
    }
}

impl CalDavClient {
    /// Create a new CalDAV client from Fastmail config
    pub async fn from_config(config: &Config) -> Result<Self> {
        let base_url = config.get_caldav_url();
        let dav_username = config.get_dav_username()?;

        let service_url = format!("{}/dav/calendars/user/{}/", base_url, dav_username);

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

        // Create libdav CalDavClient
        let uri: Uri = service_url.parse()?;
        let webdav_client = WebDavClient::new(uri, https_client);
        let client = libdav::CalDavClient::new(webdav_client);

        let inner: Box<dyn CalDavClientInner> = Box::new(CalDavClientInnerImpl { client });

        Ok(Self {
            caldav: inner,
            base_url: service_url,
        })
    }

    /// List all calendars for the user
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let home_set: Uri = self.base_url.parse()?;
        let collections = self.caldav.find_calendars(&home_set).await?;

        Ok(collections.into_iter().map(Calendar::from).collect())
    }

    /// Get a specific calendar by href
    pub async fn get_calendar(&self, href: &str) -> Result<Calendar> {
        let calendars = self.list_calendars().await?;

        calendars
            .into_iter()
            .find(|c| c.href == href || c.href.ends_with(href))
            .ok_or_else(|| anyhow!("Calendar not found: {}", href))
    }

    /// Create a new calendar
    pub async fn create_calendar(&self, name: &str, description: Option<String>) -> Result<Calendar> {
        let href = format!("{}{}/", self.base_url.trim_end_matches('/'), name);

        self.caldav.create_calendar(&href, name).await?;

        Ok(Calendar {
            href: href.clone(),
            display_name: Some(name.to_string()),
            description,
            color: None,
        })
    }

    /// Delete a calendar
    pub async fn delete_calendar(&self, href: &str) -> Result<()> {
        self.caldav.delete_resource(href).await
    }

    /// List all events in a calendar
    pub async fn list_events(&self, calendar_href: &str) -> Result<Vec<CalendarEvent>> {
        let resources = self.caldav.get_calendar_resources(calendar_href).await?;

        let mut events = Vec::new();
        for resource in resources {
            if let Ok(content) = resource.content {
                if let Some(event) = Self::parse_icalendar_event(content.data.as_bytes()) {
                    events.push(event);
                }
            }
        }

        Ok(events)
    }

    /// Get a specific event by href
    pub async fn get_event(&self, event_href: &str) -> Result<CalendarEvent> {
        let resources = self.caldav.get_calendar_resources(event_href).await?;

        resources
            .into_iter()
            .find(|r| r.href == event_href || r.href.ends_with(event_href))
            .and_then(|r| r.content.ok())
            .and_then(|c| Self::parse_icalendar_event(c.data.as_bytes()))
            .ok_or_else(|| anyhow!("Event not found or could not be parsed: {}", event_href))
    }

    /// Create or update an event in a calendar
    pub async fn put_event(&self, calendar_href: &str, event: &CalendarEvent) -> Result<String> {
        // Generate href from UID
        let event_href = format!("{}/{}.ics", calendar_href.trim_end_matches('/'), event.uid);
        let icalendar = Self::serialize_icalendar_event(event)?;

        let etag = self
            .caldav
            .put_resource(&event_href, icalendar, "text/calendar")
            .await?;

        Ok(etag.unwrap_or_default())
    }

    /// Delete an event
    pub async fn delete_event(&self, event_href: &str) -> Result<()> {
        self.caldav.delete_resource(event_href).await
    }

    /// Parse an iCalendar VEVENT from bytes (simplified MVP implementation)
    pub fn parse_icalendar_event(data: &[u8]) -> Option<CalendarEvent> {
        let content = String::from_utf8(data.to_vec()).ok()?;

        // Very simplified iCalendar parser - just extracts basic fields
        let mut uid = None;
        let mut summary = None;
        let mut description = None;
        let mut start = None;
        let mut end = None;
        let mut location = None;
        let mut status = None;

        let mut in_vevent = false;

        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("BEGIN:VEVENT") {
                in_vevent = true;
                continue;
            }

            if line.starts_with("END:VEVENT") {
                break;
            }

            if !in_vevent {
                continue;
            }

            // Simple key-value parsing (ignoring folded lines for MVP)
            if let Some((key, value)) = line.split_once(':') {
                match key {
                    "UID" => uid = Some(value.to_string()),
                    "SUMMARY" => summary = Some(value.to_string()),
                    "DESCRIPTION" => description = Some(value.to_string()),
                    "LOCATION" => location = Some(value.to_string()),
                    "STATUS" => status = Some(value.to_string()),
                    "DTSTART" => {
                        if let Ok(dt) = Self::parse_ical_datetime(value) {
                            start = Some(dt);
                        }
                    }
                    "DTEND" => {
                        if let Ok(dt) = Self::parse_ical_datetime(value) {
                            end = Some(dt);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Validate required fields
        let uid = uid?;
        let summary = summary.unwrap_or_default();
        let start = start?;

        // Default end time to start + 1 hour if not specified
        let end = end.unwrap_or_else(|| start + chrono::Duration::hours(1));

        Some(CalendarEvent {
            uid,
            summary,
            description,
            start,
            end,
            location,
            status,
        })
    }

    /// Parse an iCalendar datetime string (simplified MVP implementation)
    fn parse_ical_datetime(s: &str) -> Result<DateTime<Utc>> {
        // Simplified: only handles basic formats like "20240115T100000Z"
        let s = s.trim().trim_end_matches('Z');

        if s.len() == 15 && s.contains('T') {
            // Format: YYYYMMDDTHHMMSS
            let year = s[0..4].parse::<i32>()?;
            let month = s[4..6].parse::<u32>()?;
            let day = s[6..8].parse::<u32>()?;
            let hour = s[9..11].parse::<u32>()?;
            let minute = s[11..13].parse::<u32>()?;
            let second = s[13..15].parse::<u32>()?;

            let naive_date = chrono::NaiveDate::from_ymd_opt(year, month, day)
                .ok_or_else(|| anyhow!("Invalid date: {}-{}-{}", year, month, day))?;
            let naive_datetime = naive_date
                .and_hms_opt(hour, minute, second)
                .ok_or_else(|| anyhow!("Invalid time: {}:{}:{}", hour, minute, second))?;

            Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime, Utc))
        } else {
            Err(anyhow!("Unsupported datetime format: {}", s))
        }
    }

    /// Serialize a calendar event to iCalendar format (simplified MVP implementation)
    fn serialize_icalendar_event(event: &CalendarEvent) -> Result<String> {
        let mut ical = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//fastmail-cli//EN\r\nBEGIN:VEVENT\r\n");

        ical.push_str(&format!("UID:{}\r\n", event.uid));
        ical.push_str(&format!("SUMMARY:{}\r\n", event.summary));

        if let Some(ref desc) = event.description {
            ical.push_str(&format!("DESCRIPTION:{}\r\n", desc));
        }

        ical.push_str(&format!(
            "DTSTART:{}\r\n",
            event.start.format("%Y%m%dT%H%M%SZ")
        ));
        ical.push_str(&format!(
            "DTEND:{}\r\n",
            event.end.format("%Y%m%dT%H%M%SZ")
        ));

        if let Some(ref location) = event.location {
            ical.push_str(&format!("LOCATION:{}\r\n", location));
        }

        if let Some(ref status) = event.status {
            ical.push_str(&format!("STATUS:{}\r\n", status));
        }

        ical.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");

        Ok(ical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use chrono::Timelike;

    #[test]
    fn test_parse_icalendar_event() {
        let icalendar = b"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Test//EN
BEGIN:VEVENT
UID:test-event-123
SUMMARY:Test Meeting
DESCRIPTION:This is a test meeting
DTSTART:20240115T100000Z
DTEND:20240115T110000Z
LOCATION:Conference Room A
STATUS:CONFIRMED
END:VEVENT
END:VCALENDAR";

        let event = CalDavClient::parse_icalendar_event(icalendar).unwrap();

        assert_eq!(event.uid, "test-event-123");
        assert_eq!(event.summary, "Test Meeting");
        assert_eq!(event.description, Some("This is a test meeting".to_string()));
        assert_eq!(event.location, Some("Conference Room A".to_string()));
        assert_eq!(event.status, Some("CONFIRMED".to_string()));
    }

    #[test]
    fn test_parse_ical_datetime() {
        let dt = CalDavClient::parse_ical_datetime("20240115T100000Z").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 0);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn test_serialize_icalendar_event() {
        let event = CalendarEvent {
            uid: "test-123".to_string(),
            summary: "Test Event".to_string(),
            description: Some("Test Description".to_string()),
            start: DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            end: DateTime::parse_from_rfc3339("2024-01-15T11:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            location: Some("Office".to_string()),
            status: Some("CONFIRMED".to_string()),
        };

        let icalendar = CalDavClient::serialize_icalendar_event(&event).unwrap();

        assert!(icalendar.contains("UID:test-123"));
        assert!(icalendar.contains("SUMMARY:Test Event"));
        assert!(icalendar.contains("DESCRIPTION:Test Description"));
        assert!(icalendar.contains("DTSTART:20240115T100000Z"));
        assert!(icalendar.contains("DTEND:20240115T110000Z"));
        assert!(icalendar.contains("LOCATION:Office"));
        assert!(icalendar.contains("STATUS:CONFIRMED"));
    }

    #[test]
    fn test_parse_icalendar_event_minimal() {
        // Test with minimal required fields
        let icalendar = b"BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:minimal-event
SUMMARY:Minimal Event
DTSTART:20240115T100000Z
END:VEVENT
END:VCALENDAR";

        let event = CalDavClient::parse_icalendar_event(icalendar).unwrap();

        assert_eq!(event.uid, "minimal-event");
        assert_eq!(event.summary, "Minimal Event");
        assert!(event.description.is_none());
        assert!(event.location.is_none());
        assert!(event.status.is_none());
        // End time should default to start + 1 hour
        assert_eq!(event.end, event.start + chrono::Duration::hours(1));
    }
}
