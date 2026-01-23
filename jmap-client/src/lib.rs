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
    // Session
    Session, AccountData, CoreCapability,
    // Filter/Sort
    Filter, FilterOperator, Comparator,
    // Response types
    SetResponse, SetError, ChangesResponse, QueryChangesResponse, AddedItem,
    // Mailbox
    Mailbox, MailboxRights, MailboxFilterCondition,
    // Thread
    Thread,
    // Email
    Email, EmailAddress, BodyPart, EmailBodyPart, EmailHeader, EmailBodyValue,
    EmailFilterCondition, EmailCreate, EmailImport,
    // SearchSnippet
    SearchSnippet,
    // Identity
    Identity,
    // EmailSubmission
    EmailSubmission, Envelope, UndoStatus, DeliveryStatus,
    // VacationResponse
    VacationResponse,
    // Blob types (RFC 9404)
    BlobCapability, BlobUploadObject, DataSourceObject,
    BlobCreatedInfo, BlobUploadResponse, BlobGetResponse, BlobLookupInfo,
    BlobCopyResponse,
    // Sharing types (RFC 9670)
    Principal, PrincipalType, PrincipalFilterCondition, PrincipalSortProperty,
    ShareNotification, Entity,
    ShareNotificationFilterCondition, ShareNotificationSortProperty,
    PrincipalsCapability, PrincipalsAccountCapability, PrincipalsOwnerCapability,
    // Push
    PushSubscription, PushKeys,
};

// Re-export error types separately
pub use error::MethodError;
pub use error::error_types;

// Re-export reqwest client when feature is enabled
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;
