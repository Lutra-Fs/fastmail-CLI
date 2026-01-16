// fastmail-cli/src/commands/blob.rs
use crate::output::{print_response, ErrorResponse, Response};
use anyhow::Result;

#[derive(clap::Subcommand, Clone, Debug)]
pub enum BlobCommands {
    /// Check blob capability
    Capability,
    /// Upload a file as a blob
    Upload {
        /// Path to the file to upload
        path: String,
        /// Media type hint
        #[arg(short, long)]
        type_: Option<String>,
    },
    /// Download blob content
    Download {
        /// Blob ID
        blob_id: String,
        /// Output file path
        #[arg(short, long)]
        output: String,
    },
    /// Get blob metadata
    Info {
        /// Blob ID
        blob_id: String,
    },
    /// Look up references to a blob
    Lookup {
        /// Blob ID
        blob_id: String,
        /// Type names to search (comma-separated)
        #[arg(long, value_delimiter = ',')]
        types: Vec<String>,
    },
}

pub async fn handle_blob_command(
    client: &fastmail_client::FastmailClient,
    cmd: BlobCommands,
) -> Result<()> {
    match cmd {
        BlobCommands::Capability => {
            if client.has_blob_capability() {
                let cap = client.blob_capability();
                let resp = Response::ok(serde_json::json!({
                    "supported": true,
                    "capability": cap
                }));
                print_response(&resp)?;
            } else {
                let resp = Response::ok(serde_json::json!({
                    "supported": false,
                    "capability": null
                }));
                print_response(&resp)?;
            }
            Ok(())
        }
        BlobCommands::Upload { path, type_ } => {
            let content = tokio::fs::read(&path).await?;
            let blob_id = client
                .jmap_client()
                .blob_upload_bytes(&content, type_.as_deref())
                .await?;

            let resp = Response::ok(serde_json::json!({
                "blobId": blob_id,
                "size": content.len()
            }));
            print_response(&resp)?;
            Ok(())
        }
        BlobCommands::Download { blob_id, output } => {
            let data = client.jmap_client().blob_get_bytes(&blob_id).await?;
            tokio::fs::write(&output, data).await?;

            let resp = Response::ok(serde_json::json!({
                "blobId": blob_id,
                "savedTo": output
            }));
            print_response(&resp)?;
            Ok(())
        }
        BlobCommands::Info { blob_id } => {
            let results = client
                .jmap_client()
                .blob_get(&[blob_id.clone()], Some(vec!["size".to_string()]), None, None)
                .await?;

            if let Some(info) = results.first() {
                let resp = Response::ok(serde_json::json!({
                    "blobId": info.id,
                    "size": info.size,
                    "isEncodingProblem": info.is_encoding_problem,
                    "isTruncated": info.is_truncated
                }));
                print_response(&resp)?;
            } else {
                let resp = Response::<()>::error(ErrorResponse::not_found(
                    format!("Blob not found: {}", blob_id)
                ));
                print_response(&resp)?;
            }
            Ok(())
        }
        BlobCommands::Lookup { blob_id, types } => {
            let results = client
                .jmap_client()
                .blob_lookup(&[blob_id.clone()], &types)
                .await?;

            if let Some(info) = results.first() {
                let resp = Response::ok(serde_json::json!({
                    "blobId": info.id,
                    "matchedIds": info.matched_ids
                }));
                print_response(&resp)?;
            } else {
                let resp = Response::<()>::error(ErrorResponse::not_found(
                    format!("Blob not found: {}", blob_id)
                ));
                print_response(&resp)?;
            }
            Ok(())
        }
    }
}
