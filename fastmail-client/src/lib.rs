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
pub use config::{AccountConfig, Config, DavEndpoints};
pub use dav::{depth_from_u8, DavClient, DavResource, DavService, DepthValue};
pub use masked_email::{MaskedEmail, MaskedEmailState};
pub use whitelist::Whitelist;

// Re-export from jmap-client
pub use jmap_client::{BlobCapability, Mailbox};
// Sharing types
pub use jmap_client::{
    Entity, Principal, PrincipalFilterCondition, PrincipalType, PrincipalsAccountCapability,
    PrincipalsOwnerCapability, ShareNotification, ShareNotificationFilterCondition,
};
