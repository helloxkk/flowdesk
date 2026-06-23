// FlowDesk application settings + persistence.
//
// Stored as JSON at ~/Library/Application Support/com.flowdesk.app/config.json
// (per design doc §3.4). Mirrors the field set of the legacy AppConfig so the
// mental model is familiar, but does NOT reuse Qt's binary plist format.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::ServerConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub screen_name: String,
    pub port: u16,
    pub interface: String,
    pub log_level: String,
    pub log_to_file: bool,
    pub log_filename: String,
    pub language: String,
    pub crypto_enabled: bool,
    pub require_client_certificate: bool,
    pub auto_hide: bool,
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub enable_drag_and_drop: bool,
    pub server_config: ServerConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let screen_name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "flowdesk".into());
        Self {
            screen_name,
            port: 24800,
            interface: String::new(),
            log_level: "INFO".into(),
            log_to_file: false,
            log_filename: String::new(),
            language: "en".into(),
            crypto_enabled: true,
            require_client_certificate: false,
            auto_hide: false,
            auto_start: false,
            minimize_to_tray: false,
            enable_drag_and_drop: true,
            server_config: ServerConfig::default(),
        }
    }
}

const BUNDLE_ID: &str = "com.flowdesk.app";
const CONFIG_FILENAME: &str = "config.json";
const SERVER_CONFIG_FILENAME: &str = "server.conf";

/// Path to the JSON application settings file.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(BUNDLE_ID)
}

fn config_file() -> PathBuf {
    config_dir().join(CONFIG_FILENAME)
}

/// Path the barriers binary will read its `-c` config from.
pub fn server_config_path() -> String {
    config_dir().join(SERVER_CONFIG_FILENAME).to_string_lossy().into_owned()
}

/// Load settings; fall back to defaults (and persist them) on first run.
pub fn load_config() -> std::io::Result<AppConfig> {
    let path = config_file();
    if !path.exists() {
        let cfg = AppConfig::default();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let data = std::fs::read_to_string(&path)?;
    match serde_json::from_str::<AppConfig>(&data) {
        Ok(cfg) => Ok(cfg),
        Err(e) => {
            log::warn!("config parse error ({e}); resetting to defaults");
            let cfg = AppConfig::default();
            let _ = save_config(&cfg);
            Ok(cfg)
        }
    }
}

pub fn save_config(cfg: &AppConfig) -> std::io::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(config_file(), json)
}
