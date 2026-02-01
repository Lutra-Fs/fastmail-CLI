mod commands;
mod output;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{
    handle_calendar, handle_config, handle_contacts, handle_files, handle_mail, handle_mailbox,
    handle_masked, run_setup, CalendarCommands, ConfigCommands, ContactsCommands, FilesCommands,
    MailCommands, MailboxCommands, MaskedCommands, SharingCommands,
};
use utils::load_jmap_client;

#[derive(Parser)]
#[command(name = "fastmail")]
#[command(about = "A command-line interface for Fastmail", long_about = None)]
struct Cli {
    /// Output format: auto, json, human (default: auto)
    #[arg(short = 'o', long, global = true, value_name = "FORMAT")]
    output: Option<String>,

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
    /// Calendar operations
    #[command(subcommand)]
    Calendar(CalendarCommands),
    /// Files operations
    #[command(subcommand)]
    Files(FilesCommands),
    /// Sharing operations (JMAP RFC 9670)
    #[command(subcommand)]
    Sharing(SharingCommands),
    /// Configuration
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Setup Fastmail CLI credentials
    Setup,
}

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    let cli = Cli::parse();

    match cli.command {
        Commands::Mail(cmd) => {
            let client = load_jmap_client().await?;
            handle_mail(&client, cmd).await
        }
        Commands::Mailbox(cmd) => {
            let client = load_jmap_client().await?;
            handle_mailbox(&client, cmd).await
        }
        Commands::Blob(cmd) => {
            let client = load_jmap_client().await?;
            commands::blob::handle_blob_command(&client, cmd).await
        }
        Commands::Masked(cmd) => {
            let client = load_jmap_client().await?;
            handle_masked(&client, cmd).await
        }
        Commands::Contacts(cmd) => handle_contacts(cmd).await,
        Commands::Calendar(cmd) => handle_calendar(cmd).await,
        Commands::Files(cmd) => handle_files(cmd).await,
        Commands::Sharing(cmd) => {
            let client = load_jmap_client().await?;
            commands::sharing::handle_sharing_command(&client, cmd).await
        }
        Commands::Config(cmd) => handle_config(cmd).await,
        Commands::Setup => {
            let exit_code = run_setup().await?;
            std::process::exit(exit_code);
        }
    }
}
