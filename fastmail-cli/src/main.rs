mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use fastmail_client::{FastmailClient, MaskedEmailState};
use output::{print_response, ErrorResponse, ExitCode, Meta, Response};
use std::env;

async fn load_client() -> Result<FastmailClient> {
    let token = env::var("FASTMAIL_TOKEN")
        .or_else(|_| -> Result<String> {
            Err(anyhow::anyhow!(
                "FASTMAIL_TOKEN environment variable not set"
            ))
        })?;

    FastmailClient::new(token).await
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
    /// Masked email management
    #[command(subcommand)]
    Masked(MaskedCommands),
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
    },
    /// Delete emails
    Delete {
        /// Email ID(s) to delete
        #[arg(required = true)]
        ids: Vec<String>,
        /// Confirm destructive operation
        #[arg(long, required = true)]
        force: bool,
        /// Confirm intent (must contain email IDs)
        #[arg(long, required = true)]
        confirm: String,
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
        #[arg(long, required = true)]
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
        Commands::Masked(cmd) => handle_masked(cmd).await,
        Commands::Config(cmd) => handle_config(cmd).await,
    }
}

async fn handle_mail(cmd: MailCommands) -> Result<()> {
    match cmd {
        MailCommands::List { mailbox: _, limit } => {
            let client = load_client().await?;
            let emails = client.list_emails(limit).await?;

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
        MailCommands::Read { id: _ } => {
            // Placeholder
            Ok(())
        }
        MailCommands::Delete {
            ids,
            force,
            confirm,
            dry_run,
        } => {
            // Safety check: force flag must be true
            if !force {
                let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                    "--force flag is required for delete operations".to_string()
                ));
                print_response(&resp)?;
                std::process::exit(ExitCode::SafetyRejected.code());
            }

            // Safety check: confirm must contain all email IDs
            for id in &ids {
                if !confirm.contains(id) {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(format!(
                        "--confirm must contain email ID '{}'. Use: --confirm 'delete-{}'",
                        id, id
                    )));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                // Show what would be deleted
                println!("Would delete: {:?}", ids);
                Ok(())
            } else {
                // Actually delete
                println!("Deleted: {:?}", ids);
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
            if !force {
                let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                    "--force flag is required for delete operations".to_string()
                ));
                print_response(&resp)?;
                std::process::exit(ExitCode::SafetyRejected.code());
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
