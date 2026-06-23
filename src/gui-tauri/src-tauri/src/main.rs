// FlowDesk — Tauri GUI for the Barrier fork.
// Copyright (C) 2026 helloxkk (FlowDesk)
// Copyright (C) 2018 Debauchee Open Source Group
// Licensed under GPLv2; see LICENSE at the repo root.

// Prevents an additional console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod supervisor;
mod config;
mod logparse;
mod settings;
mod commands;
mod tray;
mod macos;

use tauri::Manager;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::start_server,
            commands::stop_server,
            commands::get_status,
            commands::get_app_config,
            commands::save_app_config,
            commands::get_server_config,
            commands::save_server_config,
            commands::get_local_ips,
            commands::check_accessibility,
            commands::request_accessibility,
            commands::open_accessibility_settings,
            commands::check_screen_capture,
            commands::request_screen_capture,
            commands::open_screen_recording_settings,
        ])
        .setup(|app| {
            log::info!("FlowDesk GUI starting");

            // Boot the restart dispatcher (one-shot at startup).
            let state: tauri::State<commands::AppState> = app.state();
            state.supervisor.start_dispatcher(app.handle().clone());

            // System tray (menu + icon + click toggle).
            if let Err(e) = tray::create_tray(app.handle()) {
                log::warn!("failed to create tray icon: {e}");
            }

            // Accessibility gate: log current status but do not block — the
            // frontend will surface a guidance prompt if not trusted.
            if !macos::is_accessible() {
                log::warn!("Accessibility permission not granted yet; barriers may fail to capture input");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FlowDesk");
}
