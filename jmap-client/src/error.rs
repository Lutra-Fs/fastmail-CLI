// Placeholder for error module - to be implemented in Task 3

// Temporary empty error type to satisfy the exports in lib.rs
// This will be properly implemented in Task 3

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlobError {
    #[error("Blob error not yet implemented")]
    Unimplemented,
}
