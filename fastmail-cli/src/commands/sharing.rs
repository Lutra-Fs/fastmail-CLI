// fastmail-cli/src/commands/sharing.rs
use crate::output::{print_response, ErrorResponse, Response};
use anyhow::Result;
use fastmail_client::{
    PrincipalType,
    PrincipalFilterCondition, ShareNotificationFilterCondition,
};

#[derive(clap::Subcommand, Clone, Debug)]
pub enum SharingCommands {
    /// Check principals capability
    Capability,
    /// List all principals
    ListPrincipals {
        /// Filter by name
        #[arg(short, long)]
        name: Option<String>,
        /// Filter by type
        #[arg(long)]
        type_: Option<String>,
        /// Limit results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Get a specific principal
    GetPrincipal {
        /// Principal ID
        id: String,
    },
    /// List share notifications
    ListNotifications {
        /// Filter by object type
        #[arg(long)]
        object_type: Option<String>,
        /// Limit results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Dismiss share notifications
    DismissNotifications {
        /// Notification IDs (comma-separated)
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
    },
}

pub async fn handle_sharing_command(
    client: &fastmail_client::FastmailClient,
    cmd: SharingCommands,
) -> Result<()> {
    match cmd {
        SharingCommands::Capability => {
            if client.has_principals_capability() {
                let cap = client.principals_capability();
                let owner = client.owner_capability();
                let current_id = client.current_principal_id();
                let resp = Response::ok(serde_json::json!({
                    "supported": true,
                    "capability": cap,
                    "owner": owner,
                    "currentPrincipalId": current_id,
                }));
                print_response(&resp)?;
            } else {
                let resp = Response::ok(serde_json::json!({
                    "supported": false,
                }));
                print_response(&resp)?;
            }
            Ok(())
        }
        SharingCommands::ListPrincipals { name, type_, limit } => {
            let mut filter = PrincipalFilterCondition::default();

            if let Some(n) = name {
                filter.name = Some(n);
            }
            if let Some(t) = type_ {
                filter.type_ = match t.to_lowercase().as_str() {
                    "individual" => Some(PrincipalType::Individual),
                    "group" => Some(PrincipalType::Group),
                    "resource" => Some(PrincipalType::Resource),
                    "location" => Some(PrincipalType::Location),
                    "other" => Some(PrincipalType::Other),
                    _ => None,
                };
            }

            let principals = client
                .list_principals(Some(filter), limit)
                .await?;

            let resp = Response::ok(principals);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::GetPrincipal { id } => {
            let principal = client.get_principal(&id).await?;
            let resp = Response::ok(principal);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::ListNotifications { object_type, limit } => {
            let mut filter = ShareNotificationFilterCondition::default();

            if let Some(ot) = object_type {
                filter.object_type = Some(ot);
            }

            let notifications = client
                .list_share_notifications(Some(filter), limit)
                .await?;

            let resp = Response::ok(notifications);
            print_response(&resp)?;
            Ok(())
        }
        SharingCommands::DismissNotifications { ids } => {
            if ids.is_empty() {
                let resp = Response::<()>::error(ErrorResponse::validation_failed(
                    "No notification IDs provided".to_string()
                ));
                print_response(&resp)?;
                return Ok(());
            }

            client.dismiss_share_notifications(&ids).await?;

            let resp = Response::ok(serde_json::json!({
                "dismissed": ids,
            }));
            print_response(&resp)?;
            Ok(())
        }
    }
}
