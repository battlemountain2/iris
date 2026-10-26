// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AccountConfig {
    pub id: String,
    pub display_name: String,
    pub email: String,
    #[serde(default)]
    pub maildir_path: Option<String>,
    #[serde(default)]
    pub smtp_host: Option<String>,
    #[serde(default)]
    pub smtp_port: Option<u16>,
    #[serde(default)]
    pub smtp_username: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SyncConfig {
    #[serde(default = "default_sync_command")]
    pub command: String,
    #[serde(default = "default_sync_interval")]
    pub interval_seconds: u64,
}

fn default_sync_command() -> String {
    "mbsync -a".to_string()
}

fn default_sync_interval() -> u64 {
    300
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IrisConfig {
    pub accounts: Vec<AccountConfig>,
    #[serde(default)]
    pub sync: SyncConfig,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            command: default_sync_command(),
            interval_seconds: default_sync_interval(),
        }
    }
}

impl Default for IrisConfig {
    fn default() -> Self {
        Self {
            accounts: vec![
                AccountConfig {
                    id: "demo".to_string(),
                    display_name: "Gmail (Personal)".to_string(),
                    email: "bry@gmail.com".to_string(),
                    maildir_path: None,
                    smtp_host: None,
                    smtp_port: None,
                    smtp_username: None,
                    smtp_password: None,
                },
                AccountConfig {
                    id: "work".to_string(),
                    display_name: "Gmail (Work)".to_string(),
                    email: "brayan@work.com".to_string(),
                    maildir_path: None,
                    smtp_host: None,
                    smtp_port: None,
                    smtp_username: None,
                    smtp_password: None,
                },
                AccountConfig {
                    id: "icloud".to_string(),
                    display_name: "iCloud".to_string(),
                    email: "brayan@icloud.com".to_string(),
                    maildir_path: None,
                    smtp_host: None,
                    smtp_port: None,
                    smtp_username: None,
                    smtp_password: None,
                },
            ],
            sync: SyncConfig::default(),
        }
    }
}

pub fn get_config_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("IRIS_CONFIG_DIR") {
        PathBuf::from(custom)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/iris")
    } else {
        std::env::temp_dir().join("iris_config")
    }
}

pub fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

pub fn load_or_create_config() -> IrisConfig {
    load_or_create_config_from(&get_config_file_path())
}

pub fn load_or_create_config_from(path: &Path) -> IrisConfig {
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(cfg) = toml::from_str::<IrisConfig>(&content) {
                return cfg;
            } else {
                tracing::warn!("Failed to parse config at {}, using defaults", path.display());
            }
        }
    } else {
        let default_cfg = IrisConfig::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(toml_str) = toml::to_string_pretty(&default_cfg) {
            let commented_toml = format!(
                "# Iris Configuration File\n\
                 # Location: ~/.config/iris/config.toml\n\n\
                 {toml_str}\n"
            );
            let _ = fs::write(path, commented_toml);
        }
    }
    IrisConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation_and_parse() {
        let temp_dir = std::env::temp_dir().join(format!("iris_cfg_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let cfg_path = temp_dir.join("config.toml");

        let loaded = load_or_create_config_from(&cfg_path);
        assert_eq!(loaded.accounts.len(), 3);
        assert_eq!(loaded.accounts[0].id, "demo");
        assert_eq!(loaded.accounts[1].id, "work");
        assert_eq!(loaded.accounts[2].id, "icloud");
        assert!(cfg_path.exists());

        // Test reading back modified config
        let custom_toml = r#"
[[accounts]]
id = "proton"
display_name = "Proton Mail"
email = "bry@pm.me"
smtp_host = "127.0.0.1"
smtp_port = 1025

[sync]
command = "mbsync proton"
interval_seconds = 120
"#;
        fs::write(&cfg_path, custom_toml).unwrap();
        let custom_loaded = load_or_create_config_from(&cfg_path);
        assert_eq!(custom_loaded.accounts.len(), 1);
        assert_eq!(custom_loaded.accounts[0].id, "proton");
        assert_eq!(custom_loaded.accounts[0].email, "bry@pm.me");
        assert_eq!(custom_loaded.accounts[0].smtp_port, Some(1025));
        assert_eq!(custom_loaded.sync.command, "mbsync proton");
        assert_eq!(custom_loaded.sync.interval_seconds, 120);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
