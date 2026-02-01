// fastmail-client/src/config.rs
use anyhow::{anyhow, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub account: AccountConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
    /// Account ID for DAV operations (retrieved from JMAP session)
    #[serde(default)]
    pub account_id: Option<String>,
    /// Authentication token for JMAP API
    #[serde(default)]
    pub token: String,
    /// App password for DAV operations (CalDAV/CardDAV/WebDAV)
    #[serde(default)]
    pub dav_password: Option<String>,
    /// DAV endpoint configuration
    #[serde(default)]
    pub dav_endpoints: Option<DavEndpoints>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct AccountConfig {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SafetyConfig {
    #[serde(default = "default_require_new_recipient_flag")]
    pub require_new_recipient_flag: bool,
    #[serde(default = "default_require_confirm")]
    pub require_confirm: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_new_recipient_flag: true,
            require_confirm: true,
        }
    }
}

fn default_require_new_recipient_flag() -> bool {
    true
}

fn default_require_confirm() -> bool {
    true
}

/// DAV endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DavEndpoints {
    #[serde(default = "default_caldav_url")]
    pub caldav: String,
    #[serde(default = "default_carddav_url")]
    pub carddav: String,
    #[serde(default = "default_webdav_url")]
    pub webdav: String,
}

fn default_caldav_url() -> String {
    "https://caldav.fastmail.com".to_string()
}

fn default_carddav_url() -> String {
    "https://carddav.fastmail.com".to_string()
}

fn default_webdav_url() -> String {
    "https://www.fastmail.com".to_string()
}

impl Default for DavEndpoints {
    fn default() -> Self {
        Self {
            caldav: default_caldav_url(),
            carddav: default_carddav_url(),
            webdav: default_webdav_url(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = Self::config_dir()?;

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        let config_path = config_dir.join("config.toml");

        if !config_path.exists() {
            let default = Self::default();
            default.save()?;
            return Ok(default);
        }

        let content = fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&content)?;

        // Allow DAV credentials to be overridden by environment variables
        if let Ok(dav_password) = std::env::var("FASTMAIL_DAV_PASSWORD") {
            config.dav_password = Some(dav_password);
        }

        // Allow email to be overridden by environment variable
        if let Ok(email) = std::env::var("FASTMAIL_EMAIL") {
            config.account.email = Some(email);
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.toml");

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        // Set permissions to 600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
        }

        Ok(())
    }

    fn config_dir() -> Result<PathBuf> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| anyhow!("Cannot determine config directory"))?;
        Ok(base_dirs.config_dir().join("fastmail-cli"))
    }

    pub fn account_email(&self) -> Option<&str> {
        self.account.email.as_deref()
    }

    /// Get the username for DAV authentication (email address)
    /// DAV endpoints use HTTP Basic Auth with email as username
    pub fn get_dav_username(&self) -> Result<&str> {
        self.account.email.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "DAV username (email) not set. Please set FASTMAIL_EMAIL environment variable"
            )
        })
    }

    /// Get the CalDAV base URL
    pub fn get_caldav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.caldav.clone())
            .unwrap_or_else(default_caldav_url)
    }

    /// Get the CardDAV base URL
    pub fn get_carddav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.carddav.clone())
            .unwrap_or_else(default_carddav_url)
    }

    /// Get the WebDAV base URL
    pub fn get_webdav_url(&self) -> String {
        self.dav_endpoints
            .as_ref()
            .map(|d| d.webdav.clone())
            .unwrap_or_else(default_webdav_url)
    }
}
