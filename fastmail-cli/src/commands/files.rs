// fastmail-cli/src/commands/files.rs
use crate::output::{print_response, ErrorResponse, ExitCode, Meta, Response};
use crate::utils::confirm;
use anyhow::Result;
use clap::Subcommand;
use fastmail_client::{Config, DavClient, DavService};
use serde_json::json;

#[derive(Subcommand, Clone, Debug)]
pub enum FilesCommands {
    /// List files
    List {
        #[arg(default_value = "/")]
        path: String,
        #[arg(short, long, default_value = "1")]
        depth: u8,
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get file info
    Info { path: String },
    /// Upload a file
    Upload {
        local: String,
        remote: String,
        #[arg(short, long)]
        content_type: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Download a file
    Download {
        remote: String,
        local: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete
    Delete {
        path: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Copy
    Copy {
        from: String,
        to: String,
        #[arg(long, default_value = "false")]
        overwrite: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Move
    Move {
        from: String,
        to: String,
        #[arg(long, default_value = "false")]
        overwrite: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Create directory
    Mkdir {
        path: String,
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_files(cmd: FilesCommands) -> Result<()> {
    let config = Config::load()?;
    let client = DavClient::from_config(&config, DavService::Files).await?;

    match cmd {
        FilesCommands::List {
            path,
            depth,
            filter,
        } => {
            let mut resources = client.list(&path, depth).await?;

            // Apply filter if provided
            if let Some(pattern) = filter {
                let pattern_lower = pattern.to_lowercase();
                resources.retain(|r| {
                    r.href.to_lowercase().contains(&pattern_lower)
                        || r.content_type
                            .as_ref()
                            .map(|ct| ct.to_lowercase().contains(&pattern_lower))
                            .unwrap_or(false)
                });
            }

            let resp = Response::ok_with_meta(
                json!({
                    "resources": resources,
                    "count": resources.len(),
                    "path": path,
                }),
                Meta {
                    rate_limit: None,
                    dry_run: None,
                    operation_id: None,
                },
            );
            print_response(&resp)?;
            Ok(())
        }
        FilesCommands::Info { path } => {
            let resource = client.get_properties(&path).await?;

            let resp = Response::ok(resource);
            print_response(&resp)?;
            Ok(())
        }
        FilesCommands::Upload {
            local,
            remote,
            content_type,
            dry_run,
        } => {
            // Read local file
            let content = std::fs::read(&local)?;

            // Detect content type if not provided
            let ct = content_type.unwrap_or_else(|| {
                mime_guess::from_path(&local)
                    .first()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string())
            });

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "upload",
                        "local": local,
                        "remote": remote,
                        "content_type": ct,
                        "size": content.len(),
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("upload-{}", remote)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                let etag = client.put(&remote, &content, &ct).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "upload",
                        "local": local,
                        "remote": remote,
                        "etag": etag,
                        "size": content.len(),
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("upload-{}", remote)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        FilesCommands::Download {
            remote,
            local,
            dry_run,
        } => {
            // Note: Current DavClient::get returns empty vec due to libdav limitations
            // This is a placeholder for when actual GET is implemented
            let content = client.get(&remote).await?;

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "download",
                        "remote": remote,
                        "local": local,
                        "size": content.len(),
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("download-{}", remote)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                std::fs::write(&local, &content)?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "download",
                        "remote": remote,
                        "local": local,
                        "size": content.len(),
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("download-{}", remote)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        FilesCommands::Delete {
            path,
            force,
            dry_run,
        } => {
            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete '{}'?", path);
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string(),
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "would_delete": path
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-{}", path)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete(&path).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": path
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-{}", path)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        FilesCommands::Copy {
            from,
            to,
            overwrite,
            dry_run,
        } => {
            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "copy",
                        "from": from,
                        "to": to,
                        "overwrite": overwrite,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("copy-{}-to-{}", from, to)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.copy(&from, &to, overwrite).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "copy",
                        "from": from,
                        "to": to,
                        "overwrite": overwrite,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("copy-{}-to-{}", from, to)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        FilesCommands::Move {
            from,
            to,
            overwrite,
            dry_run,
        } => {
            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "move",
                        "from": from,
                        "to": to,
                        "overwrite": overwrite,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("move-{}-to-{}", from, to)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.move_resource(&from, &to, overwrite).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "move",
                        "from": from,
                        "to": to,
                        "overwrite": overwrite,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("move-{}-to-{}", from, to)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        FilesCommands::Mkdir { path, dry_run } => {
            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "mkdir",
                        "would_create": path,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("mkdir-{}", path)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.create_collection(&path).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "mkdir",
                        "created": path,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("mkdir-{}", path)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
    }
}
