// FlowDesk Tauri command layer — functions exposed to the frontend via `invoke`.
// See docs/design/tauri-gui.md §3.5 for the command catalog.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use std::sync::Arc;

use tauri::State;

use crate::config::ServerConfig;
use crate::settings::{load_config, save_config, AppConfig};
use crate::supervisor::{State as SupState, Supervisor};

/// Shared app state: a single supervisor + last-loaded config.
#[derive(Default)]
pub struct AppState {
    pub supervisor: Arc<Supervisor>,
}

#[tauri::command]
pub fn ping() -> String {
    "flowdesk-tauri-alive".into()
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> SupState {
    state.supervisor.current_state()
}

#[tauri::command]
pub async fn start_server(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config = load_config().map_err(|e| e.to_string())?;
    // Reconcile the grid so screens length always matches columns × rows
    // before serializing it for barriers to consume.
    config.server_config.reconcile();

    let binary = crate::supervisor::default_binary();
    let server_path = crate::settings::server_config_path();

    // Persist the current server config to disk so barriers can read it.
    let server_cfg_text = config.server_config.to_barrier_config();
    std::fs::write(&server_path, server_cfg_text).map_err(|e| e.to_string())?;

    state
        .supervisor
        .clone()
        .start(app, config, binary, server_path)
        .await
}

#[tauri::command]
pub async fn stop_server(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.supervisor.stop(app).await
}

#[tauri::command]
pub fn get_app_config() -> Result<AppConfig, String> {
    load_config().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_app_config(config: AppConfig) -> Result<(), String> {
    save_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_server_config() -> Result<ServerConfig, String> {
    let cfg = load_config().map_err(|e| e.to_string())?;
    Ok(cfg.server_config)
}

#[tauri::command]
pub fn save_server_config(config: ServerConfig) -> Result<(), String> {
    let mut app = load_config().map_err(|e| e.to_string())?;
    app.server_config = config;
    save_config(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_local_ips() -> Result<Vec<String>, String> {
    // Best-effort local IPv4 enumeration (matches legacy getIPAddresses intent).
    let mut out = Vec::new();
    if let Ok(interfaces) = get_if_addrs::get_if_addrs() {
        for iface in interfaces {
            if let get_if_addrs::IfAddr::V4(v4) = iface.addr {
                // Skip loopback; keep everything else.
                if !v4.is_loopback() {
                    out.push(v4.ip.to_string());
                }
            }
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn check_accessibility() -> bool {
    crate::macos::is_accessible()
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    crate::macos::prompt_for_access()
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    crate::macos::open_accessibility_settings().map_err(|e| e.to_string())
}
