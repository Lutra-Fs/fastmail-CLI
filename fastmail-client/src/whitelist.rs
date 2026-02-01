// fastmail-client/src/whitelist.rs
use anyhow::{anyhow, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Whitelist {
    pub allowed_recipients: Vec<String>,
}

impl Whitelist {
    pub fn load() -> Result<Self> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| anyhow!("Cannot determine config directory"))?;

        let config_dir = base_dirs.config_dir().join("fastmail-cli");
        let whitelist_path = config_dir.join("allowed-recipients.json");

        if !whitelist_path.exists() {
            fs::create_dir_all(&config_dir)?;
            let default = Self::default();
            fs::write(&whitelist_path, serde_json::to_vec_pretty(&default)?)?;
            set_owner_only_permissions(&whitelist_path)?;
            return Ok(default);
        }

        let content = fs::read_to_string(&whitelist_path)?;
        let whitelist: Whitelist = serde_json::from_str(&content)?;
        Ok(whitelist)
    }

    pub fn is_allowed(&self, email: &str) -> bool {
        self.allowed_recipients.iter().any(|r| r == email)
    }

    pub fn add(&mut self, email: String) -> Result<()> {
        if self.is_allowed(&email) {
            return Ok(());
        }
        self.allowed_recipients.push(email);
        self.save()
    }

    pub fn remove(&mut self, email: &str) -> Result<()> {
        self.allowed_recipients.retain(|r| r != email);
        self.save()
    }

    pub fn list(&self) -> &[String] {
        &self.allowed_recipients
    }

    fn save(&self) -> Result<()> {
        let base_dirs =
            BaseDirs::new().ok_or_else(|| anyhow!("Cannot determine config directory"))?;

        let config_dir = base_dirs.config_dir().join("fastmail-cli");
        let whitelist_path = config_dir.join("allowed-recipients.json");

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&whitelist_path, content)?;

        set_owner_only_permissions(&whitelist_path)?;

        Ok(())
    }
}

fn set_owner_only_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}
