// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod calendar;
pub mod config;
pub mod contacts;
pub mod files;
pub mod mail;
pub mod mailbox;
pub mod masked;
pub mod setup;
pub mod sharing;

pub use calendar::{handle_calendar, CalendarCommands};
pub use config::{handle_config, ConfigCommands};
pub use contacts::{handle_contacts, ContactsCommands};
pub use files::{handle_files, FilesCommands};
pub use mail::{handle_mail, MailCommands};
pub use mailbox::{handle_mailbox, MailboxCommands};
pub use masked::{handle_masked, MaskedCommands};
pub use setup::run_setup;
pub use sharing::SharingCommands;
