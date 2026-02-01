// fastmail-cli/src/commands/contacts.rs
use crate::output::{print_response, ErrorResponse, ExitCode, Meta, Response};
use crate::utils::confirm;
use anyhow::Result;
use clap::Subcommand;
use fastmail_client::{CardDavClient, Config, Contact};
use serde_json::json;

#[derive(Subcommand, Clone, Debug)]
pub enum ContactsCommands {
    /// List all address books
    ListBooks {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get a specific address book
    GetBook { href: String },
    /// Create a new address book
    CreateBook {
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete an address book
    DeleteBook {
        href: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// List contacts
    List {
        #[arg(short, long)]
        book: Option<String>,
        #[arg(short, long)]
        search: Option<String>,
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    /// Get a specific contact
    Get { href: String },
    /// Create a contact (JSON input)
    Create {
        #[arg(short, long)]
        book: String,
        #[arg(short, long)]
        data: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a contact
    Delete {
        href: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn handle_contacts(cmd: ContactsCommands) -> Result<()> {
    let config = Config::load()?;
    let client = CardDavClient::from_config(&config).await?;

    match cmd {
        ContactsCommands::ListBooks { filter } => {
            let mut address_books = client.list_address_books().await?;

            // Apply filter if provided
            if let Some(pattern) = filter {
                let pattern_lower = pattern.to_lowercase();
                address_books.retain(|ab| {
                    ab.display_name.to_lowercase().contains(&pattern_lower)
                        || ab.href.to_lowercase().contains(&pattern_lower)
                });
            }

            let resp = Response::ok_with_meta(
                json!({
                    "address_books": address_books,
                    "count": address_books.len(),
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
        ContactsCommands::GetBook { href } => {
            let address_book = client.get_address_book(&href).await?;

            let resp = Response::ok(address_book);
            print_response(&resp)?;
            Ok(())
        }
        ContactsCommands::CreateBook {
            name,
            description,
            dry_run,
        } => {
            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_address_book",
                        "would_create": name,
                        "description": description,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("create-book-{}", name)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                let address_book = client.create_address_book(&name, description).await?;

                let resp = Response::ok_with_meta(
                    address_book,
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("create-book-{}", name)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        ContactsCommands::DeleteBook {
            href,
            force,
            dry_run,
        } => {
            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete address book '{}'?", href);
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
                        "would_delete": href
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-book-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete_address_book(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": href
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-book-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        ContactsCommands::List {
            book,
            search,
            limit,
        } => {
            // Determine which address book to use
            let book_href = if let Some(ref book_name) = book {
                // Try to find the address book by name/href
                let address_books = client.list_address_books().await?;
                address_books
                    .iter()
                    .find(|ab| ab.href.ends_with(book_name) || ab.display_name == *book_name)
                    .ok_or_else(|| anyhow::anyhow!("Address book not found: {}", book_name))?
                    .href
                    .clone()
            } else {
                // Use the first available address book
                let address_books = client.list_address_books().await?;
                if address_books.is_empty() {
                    return Err(anyhow::anyhow!("No address books found"));
                }
                address_books[0].href.clone()
            };

            // Get contacts (with optional search)
            let contacts = if let Some(search_query) = search {
                client.search_contacts(&book_href, &search_query).await?
            } else {
                client.list_contacts(&book_href).await?
            };

            // Apply limit
            let contacts: Vec<Contact> = contacts.into_iter().take(limit).collect();

            let resp = Response::ok_with_meta(
                json!({
                    "contacts": contacts,
                    "count": contacts.len(),
                    "book": book_href,
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
        ContactsCommands::Get { href } => {
            let contact = client.get_contact(&href).await?;

            let resp = Response::ok(contact);
            print_response(&resp)?;
            Ok(())
        }
        ContactsCommands::Create {
            book,
            data,
            dry_run,
        } => {
            // Parse contact JSON
            let contact: Contact = serde_json::from_str(&data)?;

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_contact",
                        "would_create": contact,
                        "book": book,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("create-contact-{}", contact.uid)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                let etag = client.put_contact(&book, &contact).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_contact",
                        "contact": contact,
                        "etag": etag,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("create-contact-{}", contact.uid)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        ContactsCommands::Delete {
            href,
            force,
            dry_run,
        } => {
            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete contact '{}'?", href);
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string(),
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                // Get the contact that would be deleted
                let contact = client.get_contact(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "would_delete": contact
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-contact-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete_contact(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": href
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-contact-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
    }
}
