// jmap-client/src/lib.rs
pub mod blob;
pub mod client;
pub mod error;
pub mod http;
pub mod types;

pub use blob::{data_source_from_bytes, data_source_from_text, decode_base64, encode_base64};
pub use client::JmapClient;
pub use error::BlobError;
pub use http::{HttpClient, HttpError};
pub use types::{
    AccountData,
    AddedItem,
    // Blob types (RFC 9404)
    BlobCapability,
    BlobCopyResponse,
    BlobCreatedInfo,
    BlobGetResponse,
    BlobLookupInfo,
    BlobUploadObject,
    BlobUploadResponse,
    BodyPart,
    ChangesResponse,
    Comparator,
    CoreCapability,
    DataSourceObject,
    DeliveryStatus,
    // Email
    Email,
    EmailAddress,
    EmailBodyPart,
    EmailBodyValue,
    EmailCreate,
    EmailFilterCondition,
    EmailHeader,
    EmailImport,
    // EmailSubmission
    EmailSubmission,
    Entity,
    Envelope,
    // Filter/Sort
    Filter,
    FilterOperator,
    // Identity
    Identity,
    // Mailbox
    Mailbox,
    MailboxFilterCondition,
    MailboxRights,
    // Sharing types (RFC 9670)
    Principal,
    PrincipalFilterCondition,
    PrincipalSortProperty,
    PrincipalType,
    PrincipalsAccountCapability,
    PrincipalsCapability,
    PrincipalsOwnerCapability,
    PushKeys,
    // Push
    PushSubscription,
    QueryChangesResponse,
    // SearchSnippet
    SearchSnippet,
    // Session
    Session,
    SetError,
    // Response types
    SetResponse,
    ShareNotification,
    ShareNotificationFilterCondition,
    ShareNotificationSortProperty,
    // Thread
    Thread,
    UndoStatus,
    // VacationResponse
    VacationResponse,
};

// Re-export error types separately
pub use error::error_types;
pub use error::MethodError;

// Re-export reqwest client when feature is enabled
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;
