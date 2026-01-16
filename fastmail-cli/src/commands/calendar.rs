// fastmail-cli/src/commands/calendar.rs
use crate::output::{print_response, ErrorResponse, ExitCode, Meta, Response};
use anyhow::Result;
use clap::Subcommand;
use chrono::{DateTime, Utc};
use fastmail_client::{CalDavClient, CalendarEvent, Config};
use serde_json::json;

#[derive(Subcommand, Clone, Debug)]
pub enum CalendarCommands {
    /// List all calendars
    List {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Get a specific calendar
    Get {
        href: String,
    },
    /// Create a calendar
    Create {
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a calendar
    Delete {
        href: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// List events
    ListEvents {
        #[arg(short, long)]
        calendar: Option<String>,
        #[arg(short, long)]
        from: Option<String>,
        #[arg(short, long)]
        to: Option<String>,
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    /// Get a specific event
    GetEvent {
        href: String,
    },
    /// Create an event (JSON input)
    CreateEvent {
        #[arg(short, long)]
        calendar: String,
        #[arg(short, long)]
        data: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete an event
    DeleteEvent {
        href: String,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        dry_run: bool,
    },
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

pub async fn handle_calendar(cmd: CalendarCommands) -> Result<()> {
    let config = Config::load()?;
    let client = CalDavClient::from_config(&config).await?;

    match cmd {
        CalendarCommands::List { filter } => {
            let mut calendars = client.list_calendars().await?;

            // Apply filter if provided
            if let Some(pattern) = filter {
                let pattern_lower = pattern.to_lowercase();
                calendars.retain(|c| {
                    c.display_name
                        .as_ref()
                        .map(|dn| dn.to_lowercase().contains(&pattern_lower))
                        .unwrap_or(false)
                        || c.href.to_lowercase().contains(&pattern_lower)
                });
            }

            let resp = Response::ok_with_meta(
                json!({
                    "calendars": calendars,
                    "count": calendars.len(),
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
        CalendarCommands::Get { href } => {
            let calendar = client.get_calendar(&href).await?;

            let resp = Response::ok(calendar);
            print_response(&resp)?;
            Ok(())
        }
        CalendarCommands::Create {
            name,
            description,
            dry_run,
        } => {
            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_calendar",
                        "would_create": name,
                        "description": description,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("create-calendar-{}", name)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                let calendar = client.create_calendar(&name, description).await?;

                let resp = Response::ok_with_meta(
                    calendar,
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("create-calendar-{}", name)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        CalendarCommands::Delete {
            href,
            force,
            dry_run,
        } => {
            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete calendar '{}'?", href);
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
                        operation_id: Some(format!("delete-calendar-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete_calendar(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": href
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-calendar-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        CalendarCommands::ListEvents {
            calendar,
            from,
            to,
            limit,
        } => {
            // Determine which calendar to use
            let calendar_href = if let Some(ref cal_name) = calendar {
                // Try to find the calendar by name/href
                let calendars = client.list_calendars().await?;
                calendars
                    .iter()
                    .find(|c| {
                        c.href.ends_with(cal_name)
                            || c.display_name.as_ref().map(|dn| dn == cal_name).unwrap_or(false)
                    })
                    .ok_or_else(|| anyhow::anyhow!("Calendar not found: {}", cal_name))?
                    .href
                    .clone()
            } else {
                // Use the first available calendar
                let calendars = client.list_calendars().await?;
                if calendars.is_empty() {
                    return Err(anyhow::anyhow!("No calendars found"));
                }
                calendars[0].href.clone()
            };

            // Get events
            let mut events = client.list_events(&calendar_href).await?;

            // Filter by date range if provided
            if let Some(from_str) = from {
                if let Ok(from_dt) = DateTime::parse_from_rfc3339(&from_str) {
                    let from_utc = from_dt.with_timezone(&Utc);
                    events.retain(|e| e.start >= from_utc);
                }
            }

            if let Some(to_str) = to {
                if let Ok(to_dt) = DateTime::parse_from_rfc3339(&to_str) {
                    let to_utc = to_dt.with_timezone(&Utc);
                    events.retain(|e| e.start <= to_utc);
                }
            }

            // Apply limit
            let events: Vec<CalendarEvent> = events.into_iter().take(limit).collect();

            let resp = Response::ok_with_meta(
                json!({
                    "events": events,
                    "count": events.len(),
                    "calendar": calendar_href,
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
        CalendarCommands::GetEvent { href } => {
            let event = client.get_event(&href).await?;

            let resp = Response::ok(event);
            print_response(&resp)?;
            Ok(())
        }
        CalendarCommands::CreateEvent {
            calendar,
            data,
            dry_run,
        } => {
            // Parse event JSON
            let event: CalendarEvent = serde_json::from_str(&data)?;

            if dry_run {
                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_event",
                        "would_create": event,
                        "calendar": calendar,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("create-event-{}", event.uid)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                let etag = client.put_event(&calendar, &event).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "create_event",
                        "event": event,
                        "etag": etag,
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("create-event-{}", event.uid)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
        CalendarCommands::DeleteEvent {
            href,
            force,
            dry_run,
        } => {
            // Prompt for confirmation unless --force is specified
            if !force && !dry_run {
                let prompt = format!("Delete event '{}'?", href);
                if !confirm(&prompt)? {
                    let resp = Response::<()>::error(ErrorResponse::safety_rejected(
                        "Operation cancelled".to_string(),
                    ));
                    print_response(&resp)?;
                    std::process::exit(ExitCode::SafetyRejected.code());
                }
            }

            if dry_run {
                // Get the event that would be deleted
                let event = client.get_event(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "would_delete": event
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(true),
                        operation_id: Some(format!("delete-event-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            } else {
                client.delete_event(&href).await?;

                let resp = Response::ok_with_meta(
                    json!({
                        "operation": "delete",
                        "deleted": href
                    }),
                    Meta {
                        rate_limit: None,
                        dry_run: Some(false),
                        operation_id: Some(format!("delete-event-{}", href)),
                    },
                );
                print_response(&resp)?;
                Ok(())
            }
        }
    }
}
