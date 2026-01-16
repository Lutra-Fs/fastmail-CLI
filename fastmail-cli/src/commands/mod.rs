// fastmail-cli/src/commands/mod.rs
pub mod blob;
pub mod calendar;
pub mod contacts;
pub mod files;
pub mod setup;
pub mod sharing;

pub use calendar::{CalendarCommands, handle_calendar};
pub use contacts::{ContactsCommands, handle_contacts};
pub use files::{FilesCommands, handle_files};
pub use setup::run_setup;
pub use sharing::SharingCommands;
