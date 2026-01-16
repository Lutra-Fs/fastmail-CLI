// fastmail-cli/src/commands/setup.rs
use anyhow::Result;
use dialoguer::Password;
use fastmail_client::{Config, DavEndpoints};

/// Exit code type
pub type SetupExitCode = i32;

/// Run the interactive setup command
pub async fn run_setup() -> Result<SetupExitCode> {
    println!("Fastmail CLI Setup");
    println!();

    // Prompt for API token
    let token = Password::new()
        .with_prompt("Enter your Fastmail API token")
        .interact()?;

    if token.is_empty() {
        eprintln!("Error: API token cannot be empty");
        return Ok(2);
    }

    println!();
    println!("Validating credentials...");

    // Validate token by trying to create a client
    let validation_result = fastmail_client::FastmailClient::new(token.clone()).await;

    match validation_result {
        Ok(client) => {
            // Get the email from the session
            let email = client.account_email().to_string();

            println!();
            let config = Config {
                token,
                account: fastmail_client::AccountConfig { email: Some(email) },
                dav_endpoints: Some(DavEndpoints::default()),
                ..Default::default()
            };

            // Save config
            if let Err(e) = config.save() {
                eprintln!("Error: Couldn't write config file: {}", e);
                return Ok(2);
            }

            println!("Credentials saved!");
            println!();
            println!("Try: fastmail mail list");

            Ok(0)
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Visit https://app.fastmail.com/settings/security/integrations");
            eprintln!("to create an API token.");
            Ok(2)
        }
    }
}
