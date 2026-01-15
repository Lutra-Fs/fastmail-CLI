mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use output::{print_response, ErrorResponse, ExitCode, Meta, Response};

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
        MailCommands::List { mailbox: _, limit: _ } => {
            // Placeholder
            let resp: Response<Vec<String>> = Response::ok_with_meta(
                vec![],
                Meta {
                    rate_limit: None,
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
    match cmd {
        MaskedCommands::List { filter: _, state: _ } => {
            let resp: Response<Vec<String>> = Response::ok(vec![]);
            print_response(&resp)?;
            Ok(())
        }
        MaskedCommands::Create { domain: _, description: _, prefix: _ } => {
            Ok(())
        }
        MaskedCommands::Enable { id: _ } => {
            Ok(())
        }
        MaskedCommands::Disable { id: _ } => {
            Ok(())
        }
        MaskedCommands::Delete { id: _, force: _ } => {
            Ok(())
        }
    }
}

async fn handle_config(cmd: ConfigCommands) -> Result<()> {
    match cmd {
        ConfigCommands::AllowRecipient(cmd) => match cmd {
            AllowRecipientCommands::Add { email } => {
                let resp = Response::ok(serde_json::json!({"email": email, "added": true}));
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::List => {
                let resp = Response::ok(vec![String::new()]); // Placeholder - will be filled in Task 13
                print_response(&resp)?;
                Ok(())
            }
            AllowRecipientCommands::Remove { email } => {
                let resp = Response::ok(serde_json::json!({"email": email, "removed": true}));
                print_response(&resp)?;
                Ok(())
            }
        },
    }
}
