// fastmail-client/src/config.rs
use anyhow::{anyhow, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[derive(Default)]
pub struct Config {
    #[serde(default)]
    account: AccountConfig,
    #[serde(default)]
    safety: SafetyConfig,
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
        let config: Config = toml::from_str(&content)?;
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
        let base_dirs = BaseDirs::new()
            .ok_or_else(|| anyhow!("Cannot determine config directory"))?;
        Ok(base_dirs.config_dir().join("fastmail-cli"))
    }

    pub fn account_email(&self) -> Option<&str> {
        self.account.email.as_deref()
    }
}

