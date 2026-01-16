// jmap-client/src/lib.rs
pub mod blob;
pub mod client;
pub mod error;
pub mod http;
pub mod types;

pub use blob::{encode_base64, decode_base64, data_source_from_bytes, data_source_from_text};
pub use client::JmapClient;
pub use error::BlobError;
pub use http::{HttpClient, HttpError};
pub use types::{
    Email, EmailAddress, Mailbox, BodyPart, Session, AccountData,
    // Blob types
    BlobCapability, BlobUploadObject, DataSourceObject,
    BlobCreatedInfo, BlobUploadResponse,
    BlobGetResponse, BlobLookupInfo,
    // Sharing types (RFC 9670)
    Principal, PrincipalType, PrincipalFilterCondition, PrincipalSortProperty,
    ShareNotification, Entity,
    ShareNotificationFilterCondition, ShareNotificationSortProperty,
    PrincipalsCapability, PrincipalsAccountCapability, PrincipalsOwnerCapability,
};

// Re-export reqwest client when feature is enabled
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;
