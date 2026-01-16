pub mod caldav;
pub mod carddav;
pub mod client;
pub mod config;
pub mod dav;
pub mod masked_email;
pub mod whitelist;

pub use caldav::{CalDavClient, Calendar, CalendarEvent};
pub use carddav::{AddressBook, CardDavClient, Contact};
pub use client::FastmailClient;
pub use config::{Config, AccountConfig, DavEndpoints};
pub use dav::{DavClient, DavResource, DavService, depth_from_u8, DepthValue};
pub use masked_email::{MaskedEmail, MaskedEmailState};
pub use whitelist::Whitelist;

// Re-export from jmap-client
pub use jmap_client::{Mailbox, BlobCapability};
// Sharing types
pub use jmap_client::{
    Principal, PrincipalType, PrincipalFilterCondition,
    ShareNotification, Entity,
    ShareNotificationFilterCondition,
    PrincipalsAccountCapability, PrincipalsOwnerCapability,
};
