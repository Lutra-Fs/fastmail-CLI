mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{handle_contacts, ContactsCommands};
use fastmail_client::{FastmailClient, MaskedEmailState};
use output::{print_response, ErrorResponse, ExitCode, Meta, Response};
use serde_json::json;
use std::env;

async fn load_client() -> Result<FastmailClient> {
    let token = env::var("FASTMAIL_TOKEN")
        .map_err(|_| anyhow::anyhow!(
            "FASTMAIL_TOKEN environment variable not set"
        ))?;

    FastmailClient::new(token).await
}

/// Prompt user for confirmation, returns true if user confirms
fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N]: ", prompt);
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}

#[derive(Parser)]
#[command(name = "fastmail")]
#[command(about = "A command-line interface for Fastmail", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Email operations
    #[command(subcommand)]
    Mail(MailCommands),
    /// Mailbox operations
    #[command(subcommand)]
    Mailbox(MailboxCommands),
    /// Blob operations
    #[command(subcommand)]
    Blob(commands::blob::BlobCommands),
    /// Masked email management
    #[command(subcommand)]
    Masked(MaskedCommands),
    /// Contacts operations
    #[command(subcommand)]
    Contacts(ContactsCommands),
    /// Configuration
    #[command(subcommand)]
    Config(ConfigCommands),
}

#[derive(Subcommand)]
enum MailCommands {
    /// List emails
    List {
        /// Mailbox name [default: INBOX]
        #[arg(short, long)]
        mailbox: Option<String>,
        /// Max number of emails [default: 20]
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Read an email
    Read {
        /// Email ID
        id: String,
        /// Include email body content
        #[arg(short, long)]
        body: bool,
    },
    /// Delete emails
    Delete {
        /// Email ID(s) to delete
        #[arg(required = true)]
        ids: Vec<String>,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum MailboxCommands {
    /// List available mailboxes
    List {
        /// Filter mailboxes by name pattern (case-insensitive substring match)
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Create a new mailbox
    Create {
        /// Mailbox name
        name: String,
    },
    /// Delete a mailbox
    Delete {
        /// Mailbox ID to delete
        id: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
        /// Preview without executing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum MaskedCommands {
    /// List masked emails
    List {
        /// Filter by domain
        #[arg(short, long)]
        filter: Option<String>,
        /// Filter by state
        #[arg(short, long)]
        state: Option<String>,
    },
    /// Create a masked email
    Create {
        /// Domain for the masked email (e.g., https://example.com)
        domain: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Email prefix
        #[arg(short, long)]
        prefix: Option<String>,
    },
    /// Enable a masked email
    Enable {
        /// Masked email ID or email address
        id: String,
    },
    /// Disable a masked email
    Disable {
        /// Masked email ID or email address
        id: String,
    },
    /// Delete a masked email
    Delete {
        /// Masked email ID or email address
        id: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Manage recipient whitelist
    #[command(subcommand)]
    AllowRecipient(AllowRecipientCommands),
}

#[derive(Subcommand)]
enum AllowRecipientCommands {
    /// Add email to whitelist
    Add { email: String },
    /// List whitelist
    List,
    /// Remove email from whitelist
    Remove { email: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mail(cmd) => handle_mail(cmd).await,
        Commands::Mailbox(cmd) => handle_mailbox(cmd).await,
        Commands::Blob(cmd) => handle_blob(cmd).await,
        Commands::Masked(cmd) => handle_masked(cmd).await,
        Commands::Contacts(cmd) => handle_contacts(cmd).await,
        Commands::Config(cmd) => handle_config(cmd).await,
    }
}

async fn handle_mail(cmd: MailCommands) -> Result<()> {
    match cmd {
        MailCommands::List { mailbox, limit } => {
            let client = load_client().await?;
            let emails = client.list_emails(mailbox.as_deref(), limit).await?;

            let resp = Response::ok_with_meta(
                emails,
                Meta {
                    rate_limit: None,  // TODO: track rate limits
                    dry_run: None,
                    operation_id: None,
                },
            );
            print_response(&resp)?;
            Ok(())
        }
        MailCommands::Read { id, body } => {
            let client = load_client().await?;
            let email = if body {
                client.get_email_with_body(&id).await?
            } else {
                client.get_email(&id).await?
            };

            let resp = Response::ok(email);
            print_response(&resp)?;
            Ok(())
        }
        MailCommands::Delete {
            ids,
            force,
            dry_run,
        } => {
            let client = load_client().await?;

            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = if ids.len() == 1 {
                    format!("Delete email '{}'?", ids[0])
                } else {
                    format!("Delete {} emails?", ids.len())
                };
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string()
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                // Fetch emails that would be deleted
                let mut emails_to_delete = Vec::with_capacity(ids.len());
                for id in &ids {
                    emails_to_delete.push(client.get_email(id).await?);
                }

                let resp = Response::ok_with_meta(
                    serde_json::json!({
                        "operation": "delete",
                        "would_delete": emails_to_delete
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-{}", ids.join(","))),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                // Actually delete
                client.delete_emails(ids.clone()).await?;

                let resp = Response::ok_with_meta(
                    serde_json::json!({
                        "operation": "delete",
                        "deleted": ids
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-{}", ids.join(","))),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
    }
}

async fn handle_mailbox(cmd: MailboxCommands) -> Result<()> {
    match cmd {
        MailboxCommands::List { filter } => {
            let client = load_client().await?;
            let mailboxes = client.list_mailboxes(filter.as_deref()).await?;

            let output = json!({
                "filter": filter,
                "mailboxes": mailboxes
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        }
        MailboxCommands::Create { name } => {
            let client = load_client().await?;
            let mailbox = client.create_mailbox(&name).await?;

            let resp = Response::ok(mailbox);
            print_response(&resp)?;
            Ok(())
        }
        MailboxCommands::Delete {
            id,
            force,
            dry_run,
        } => {
            let client = load_client().await?;

            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete mailbox '{}'?", id);
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string()
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "would_delete": id
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-{}", id)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete_mailbox(&id).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": id
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-{}", id)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
    }
}

async fn handle_masked(cmd: MaskedCommands) -> Result<()> {
    let client = load_client().await?;

    match cmd {
        MaskedCommands::List { filter, state } => {
            let mut emails = client.list_masked_emails().await?;

            // Apply filters
            if let Some(domain) = filter {
                emails.retain(|e| e.for_domain == domain);
            }
            if let Some(state_str) = state {
                let state = match state_str.as_str() {
                    "pending" => MaskedEmailState::Pending,
                    "enabled" => MaskedEmailState::Enabled,
                    "disabled" => MaskedEmailState::Disabled,
                    "deleted" => MaskedEmailState::Deleted,
                    _ => return Err(anyhow::anyhow!("Invalid state: {}", state_str)),
                };
                emails.retain(|e| e.state == state);
            }

            let resp = Response::ok(emails);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Create {
            domain,
            description,
            prefix,
        } => {
            let email = client
                .create_masked_email(
                    &domain,
                    description.as_deref().unwrap_or(""),
                    prefix.as_deref(),
                )
                .await?;

            let resp = Response::ok(email);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Enable { id } => {
            client
                .set_masked_email_state(&id, MaskedEmailState::Enabled)
                .await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "enabled"}));
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Disable { id } => {
            client
                .set_masked_email_state(&id, MaskedEmailState::Disabled)
                .await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "disabled"}));
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Delete { id, force } => {
            // Prompt for confirmation unless --force is specified
            if !force {
                let prompt = format!("Delete masked email '{}'?", id);
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string()
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }
            client.set_masked_email_state(&id, MaskedEmailState::Deleted).await?;
            let resp = Response::ok(serde_json::json!({"id": id, "state": "deleted"}));
            print_response(&resp)?;
            Ok(())
        }
    }
}

async fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::AllowRecipient(allow) => match allow {
            AllowRecipientCommands::Add { email } => {
                let mut whitelist = fastmail_client::Whitelist::load()?;
                whitelist.add(email.clone())?;
                let resp = Response::ok(serde_json::json!({
                    "email": email,
                    "added": true
                }));
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::List => {
                let whitelist = fastmail_client::Whitelist::load()?;
                let resp = Response::ok(serde_json::json!({
                    "allowed_recipients": whitelist.list()
                }));
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::Remove { email } => {
                let mut whitelist = fastmail_client::Whitelist::load()?;
                whitelist.remove(&email)?;
                let resp = Response::ok(serde_json::json!({
                    "email": email,
                    "removed": true
                }));
                print_response(&resp)?;
                Ok(())
            }
        },
    }
}

async fn handle_blob(cmd: commands::blob::BlobCommands) -> Result<()> {
    let client = load_client().await?;
    commands::blob::handle_blob_command(&client, cmd).await
}
