// FlowDesk system tray.
//
// Mirrors the legacy Qt tray (MainWindow::createTrayIcon): a status icon
// that swaps between disconnected/connected/transfering, a context menu
// (Start/Stop/Show Log/Hide/Show/Quit), and double-click to toggle the
// window. See docs/design/tauri-gui.md §3.6.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime, WebviewWindow,
};

use crate::supervisor::State;

/// A non-empty Image is required to build a tray icon; Tauri does not allow
/// a None icon at creation time. We supply a 1x1 fallback that is immediately
/// replaced by set_icon_for_state.
fn fallback_icon<R: Runtime>(app: &AppHandle<R>) -> Image<'_> {
    // Use the bundled app icon as the initial tray icon.
    app.default_window_icon()
        .cloned()
        .unwrap_or_else(|| Image::new_owned(vec![0u8; 4], 1, 1))
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let start = MenuItem::with_id(app, "tray-start", "Start", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "tray-stop", "Stop", true, None::<&str>)?;
    let show_log = MenuItem::with_id(app, "tray-show-log", "Show Log", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "tray-hide", "Hide", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "tray-show", "Show", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&start, &stop, &show_log, &hide, &show, &quit],
    )?;

    let initial = fallback_icon(app);
    let _tray = TrayIconBuilder::with_id("main")
        .icon(initial)
        .tooltip("FlowDesk")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-start" => {
                emit_tray_action(app, "start");
            }
            "tray-stop" => {
                emit_tray_action(app, "stop");
            }
            "tray-show-log" => {
                emit_tray_action(app, "show-log");
            }
            "tray-hide" => {
                if let Some(w) = main_window(app) {
                    let _ = w.hide();
                }
            }
            "tray-show" => {
                if let Some(w) = main_window(app) {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "tray-quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // Double-click-style toggle: left click flips window visibility.
                let app = tray.app_handle();
                if let Some(w) = main_window(app) {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window("main")
}

fn emit_tray_action<R: Runtime>(app: &AppHandle<R>, action: &str) {
    use tauri::Emitter;
    let _ = app.emit("tray://action", action);
}

/// Swap the tray icon according to supervisor state.
/// In a full build we'd ship 3 distinct template PNGs; for now we only swap
/// the tooltip text, which already communicates state on macOS.
pub fn update_tray_state<R: Runtime>(app: &AppHandle<R>, state: State) {
    let label = match state {
        State::Stopped => "FlowDesk — Stopped",
        State::Starting => "FlowDesk — Starting…",
        State::Connected => "FlowDesk — Running",
        State::Disconnected => "FlowDesk — Disconnected",
        State::Error => "FlowDesk — Error",
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(label));
    }
}
