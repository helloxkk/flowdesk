// FlowDesk process supervisor.
//
// Mirrors the existing Qt GUI's "Desktop mode" on macOS: spawn `barriers -f`
// as a child, scrape stdout for state beacons, and stop it by writing a
// single 'S' byte to stdin. See docs/design/tauri-gui.md §3.1.
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use crate::logparse::{parse_log_line, DerivedState, LogEvent};
use crate::settings::AppConfig;

/// Message sent to the dispatcher task to request a (re)launch.
enum RestartRequest {
    Launch {
        config: AppConfig,
        binary: String,
        config_path: String,
    },
}

/// Lifecycle states. Aligned with the Qt `qBarrierState` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Stopped,
    Starting,
    Connected,
    Disconnected,
    Error,
}

impl Default for State {
    fn default() -> Self {
        State::Stopped
    }
}

/// The single byte the barriers/barrierc core reads from stdin to shut down.
/// See src/gui/src/ShutdownCh.h in the legacy GUI.
const SHUTDOWN_BYTE: u8 = b'S';

/// Auto-restart delay, matching the legacy GUI (MainWindow.cpp:807).
const AUTO_RESTART_DELAY: Duration = Duration::from_secs(1);

pub struct Supervisor {
    inner: Mutex<Inner>,
}

struct Inner {
    state: State,
    /// Expected running state: when the user clicks Start we set this true,
    /// and on unexpected exit we auto-restart. Set false on explicit Stop.
    wanted: bool,
    child: Option<Child>,
    /// Send a value to interrupt the auto-restart loop (set on stop()).
    cancel: Option<oneshot::Sender<()>>,
    /// Handle into the auto-restart watcher task so we can await it on stop.
    watcher: Option<tauri::async_runtime::JoinHandle<()>>,
    /// The barriers binary path used for the last launch.
    binary_path: Option<String>,
    /// Channel into the restart dispatcher. None until start_dispatcher() is called.
    restart_tx: Option<mpsc::UnboundedSender<RestartRequest>>,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner {
                state: State::Stopped,
                wanted: false,
                child: None,
                cancel: None,
                watcher: None,
                binary_path: None,
                restart_tx: None,
            }),
        }
    }
}

impl Supervisor {
    pub fn current_state(&self) -> State {
        self.inner.lock().unwrap().state
    }

    fn set_state(&self, app: &AppHandle, s: State) {
        let changed = {
            let mut g = self.inner.lock().unwrap();
            let changed = g.state != s;
            g.state = s;
            changed
        };
        if changed {
            log::info!("state -> {:?}", s);
            let _ = app.emit("state://change", s);
            // Also reflect state onto the system tray tooltip (if present).
            crate::tray::update_tray_state(app, s);
        }
    }

    /// Launch `barriers` as a child process with the given config.
    pub async fn start(
        self: &Arc<Self>,
        app: AppHandle,
        config: AppConfig,
        binary_path: String,
        server_config_path: String,
    ) -> Result<(), String> {
        // Abort if already running.
        {
            let g = self.inner.lock().unwrap();
            if g.child.is_some() {
                return Err("server already running".into());
            }
        }

        let args = build_server_args(&config, &server_config_path);
        log::info!("starting {} with args: {:?}", binary_path, args);

        let mut cmd = server_command(&binary_path);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "no stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "no stderr".to_string())?;

        {
            let mut g = self.inner.lock().unwrap();
            g.wanted = true;
            g.child = Some(child);
            g.binary_path = Some(binary_path.clone());
        }
        self.set_state(&app, State::Starting);

        let this = Arc::clone(self);
        let app_stdout = app.clone();
        let app_stderr = app.clone();
        // Stdout reader: parse log lines, emit events, drive state.
        tauri::async_runtime::spawn(async move {
            let app_exit = app_stdout.clone();
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                this.handle_log_line(&app_stdout, &line);
            }
            // stdout closed => process ended
            this.on_child_exit(&app_exit).await;
        });

        // Stderr reader: macOS emits some unavoidable framework noise when
        // barriers runs headless (no app bundle). Filter those out so the log
        // window stays readable; surface genuinely useful warnings.
        let this_stderr = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            let mut cursor_warned = false;
            while let Ok(Some(line)) = reader.next_line().await {
                // Drop the known macOS headless-mode framework chatter:
                //  - "[WindowTab] Cannot index window tabs due to missing main bundle identifier"
                //  - "-[NSWindow makeKeyWindow] called on <NSWindow ...>"
                //  - "starting cocoa loop"
                // These are emitted because barriers is a CLI binary without a
                // main bundle; they don't affect functionality.
                if line.contains("missing main bundle identifier")
                    || line.contains("makeKeyWindow")
                    || line.contains("starting cocoa loop")
                {
                    continue;
                }

                // "cursor may not be visible" repeats forever when barriers
                // can't grab the pointer — almost always a missing macOS
                // Accessibility grant for the BARRIERS subprocess (not the GUI
                // itself; CGEventTap permission is per-process). Emit it once,
                // surface a dedicated permission prompt carrying the binary path
                // so the frontend can show actionable guidance.
                if line.contains("cursor may not be visible") {
                    if cursor_warned {
                        continue;
                    }
                    cursor_warned = true;
                    let msg = "cursor capture failed — the barriers process needs macOS Accessibility permission";
                    log::warn!("{}", msg);
                    let _ = app_stderr.emit(
                        "log://line",
                        serde_json::json!({"level": "WARNING", "message": msg}),
                    );
                    // Tell the frontend to show the permission banner with the
                    // exact binary path the user must authorize.
                    let binary_path = {
                        let g = this_stderr.inner.lock().unwrap();
                        g.binary_path.clone().unwrap_or_else(|| "barriers".to_string())
                    };
                    match app_stderr.emit(
                        "permission://needed",
                        serde_json::json!({ "binary_path": binary_path }),
                    ) {
                        Ok(()) => log::info!("emitted permission://needed for {}", binary_path),
                        Err(e) => log::error!("failed to emit permission://needed: {e}"),
                    }
                    continue;
                }

                log::warn!("barriers stderr: {}", line);
                let _ = app_stderr.emit(
                    "log://line",
                    serde_json::json!({"level": "WARNING", "message": line}),
                );
            }
        });

        Ok(())
    }

    /// Spawn the restart dispatcher loop. Must be called once at app startup
    /// (from a Tauri setup hook). The dispatcher consumes restart requests
    /// from the channel and invokes `start` — this indirection breaks what
    /// would otherwise be a compile-time recursion cycle between `start` and
    /// `on_child_exit`.
    ///
    /// Uses `tauri::async_runtime::spawn` (not `tauri::async_runtime::spawn`) so it runs on
    /// Tauri's managed runtime — the setup hook is synchronous and has no
    /// ambient Tokio runtime context of its own.
    pub fn start_dispatcher(self: &Arc<Self>, app: AppHandle) {
        let (tx, mut rx) = mpsc::unbounded_channel::<RestartRequest>();
        self.inner.lock().unwrap().restart_tx = Some(tx);

        let this = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            while let Some(req) = rx.recv().await {
                match req {
                    RestartRequest::Launch { config, binary, config_path } => {
                        if let Err(e) = this.clone().start(app.clone(), config, binary, config_path).await {
                            log::error!("auto-restart failed: {e}");
                            let _ = app.emit(
                                "log://line",
                                serde_json::json!({"level": "ERROR", "message": format!("auto-restart failed: {e}")}),
                            );
                        }
                    }
                }
            }
        });
    }

    fn handle_log_line(&self, app: &AppHandle, raw: &str) {
        let parsed = parse_log_line(raw);
        // Always forward the line to the frontend log view.
        let payload = serde_json::json!({
            "level": parsed.level_str(),
            "message": parsed.message(),
        });
        let _ = app.emit("log://line", payload);

        for event in parsed.events() {
            match event {
                LogEvent::StateChange(s) => {
                    // Map the derived state onto our supervisor state.
                    let mapped = match s {
                        DerivedState::Connected => State::Connected,
                        DerivedState::Error => State::Error,
                    };
                    self.set_state(app, mapped);
                }
                LogEvent::FingerprintPrompt { sha1, sha256 } => {
                    let _ = app.emit(
                        "fingerprint://prompt",
                        serde_json::json!({ "sha1": sha1, "sha256": sha256 }),
                    );
                }
            }
        }
    }

    /// Called when the child's stdout EOFs (i.e. the process ended).
    async fn on_child_exit(self: &Arc<Self>, app: &AppHandle) {
        // Take the child out of the mutex so the guard is dropped before .await
        // (MutexGuard is not Send). Reap it outside the lock.
        let (child, wanted) = {
            let mut g = self.inner.lock().unwrap();
            (g.child.take(), g.wanted)
        };
        if let Some(mut child) = child {
            let _ = child.wait().await;
        }

        if !wanted {
            self.set_state(app, State::Stopped);
            return;
        }

        // Unexpected exit while wanted → auto-restart.
        log::warn!("barriers exited unexpectedly; restarting in {:?}", AUTO_RESTART_DELAY);
        self.set_state(app, State::Disconnected);

        // Watcher task: wait the delay, then request a relaunch via the
        // dispatcher channel (NOT a direct recursive call, which would
        // create a compile-time cycle in the async return-type inference).
        let (tx, rx) = oneshot::channel::<()>();
        {
            let mut g = self.inner.lock().unwrap();
            g.cancel = Some(tx);
        }
        let this = Arc::clone(self);
        let app2 = app.clone();
        let handle = tauri::async_runtime::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(AUTO_RESTART_DELAY) => {
                    let still_wanted = this.inner.lock().unwrap().wanted;
                    if !still_wanted {
                        return;
                    }
                    // Reload config + request relaunch via channel.
                    let config = crate::settings::load_config().unwrap_or_default();
                    let binary = {
                        let g = this.inner.lock().unwrap();
                        g.binary_path.clone().unwrap_or_else(default_binary_path)
                    };
                    let config_path = crate::settings::server_config_path();
                    let req = RestartRequest::Launch { config, binary, config_path };
                    let g = this.inner.lock().unwrap();
                    if let Some(ch) = &g.restart_tx {
                        let _ = ch.send(req);
                    }
                    drop(app2); // keep handle alive in case future needs it
                }
                _ = rx => {
                    // Cancelled by stop(); nothing to do.
                }
            }
        });
        self.inner.lock().unwrap().watcher = Some(handle);
    }

    /// Stop the running server (writes SHUTDOWN_BYTE then waits; force on timeout).
    pub async fn stop(&self, app: AppHandle) -> Result<(), String> {
        // Cancel any pending auto-restart.
        let cancel = {
            let mut g = self.inner.lock().unwrap();
            g.wanted = false;
            g.cancel.take()
        };
        if let Some(tx) = cancel {
            let _ = tx.send(());
        }

        // Write the shutdown byte to stdin. Take stdin out of the child so the
        // lock guard is dropped before the .await (MutexGuard is not Send).
        let stdin_taken = {
            let mut g = self.inner.lock().unwrap();
            if let Some(child) = g.child.as_mut() {
                child.stdin.take()
            } else {
                None
            }
        };

        if let Some(mut stdin) = stdin_taken {
            let _ = stdin.write_all(&[SHUTDOWN_BYTE]).await;
            let _ = stdin.shutdown().await;
            log::info!("sent shutdown byte to barriers");
        }

        let child_taken = {
            let g = self.inner.lock().unwrap();
            g.child.is_some()
        };

        if child_taken {
            // Wait up to 5s for graceful exit, then kill.
            let deadline = tokio::time::sleep(Duration::from_secs(5));
            tokio::pin!(deadline);

            let exited = loop {
                let still_alive = {
                    let g = self.inner.lock().unwrap();
                    g.child.is_some()
                };
                if !still_alive {
                    break true;
                }
                tokio::select! {
                    _ = &mut deadline => break false,
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }
            };

            if !exited {
                log::warn!("barriers did not exit in 5s; killing");
                // Take the child out of the mutex before awaiting its exit.
                let child = {
                    let mut g = self.inner.lock().unwrap();
                    g.child.take()
                };
                if let Some(mut child) = child {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }
            }
        }

        // Wait for the watcher (if any) to finish cancelling.
        let watcher = {
            let mut g = self.inner.lock().unwrap();
            g.watcher.take()
        };
        if let Some(h) = watcher {
            let _ = h.await;
        }

        self.set_state(&app, State::Stopped);
        Ok(())
    }
}

/// Build the `barriers` CLI args (see MainWindow::serverArgs in the legacy GUI).
pub fn build_server_args(config: &AppConfig, server_config_path: &str) -> Vec<String> {
    let mut args = vec![
        "-f".into(),
        "--no-tray".into(),
        "--debug".into(),
        config.log_level.as_str().into(),
        "--name".into(),
        config.screen_name.clone(),
        "-c".into(),
        server_config_path.to_string(),
    ];

    let address = if config.interface.is_empty() {
        format!(":{}", config.port)
    } else {
        format!("[{}]:{}", config.interface, config.port)
    };
    args.push("--address".into());
    args.push(address);

    if !config.crypto_enabled {
        args.push("--disable-crypto".into());
    }

    if config.enable_drag_and_drop {
        args.push("--enable-drag-drop".into());
    }

    args
}

fn server_command(binary_path: &str) -> Command {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        if binary_supports_arch(binary_path, "x86_64") {
            let mut cmd = Command::new("/usr/bin/arch");
            cmd.arg("-x86_64").arg(binary_path);
            return cmd;
        }
    }

    Command::new(binary_path)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn binary_supports_arch(binary_path: &str, arch: &str) -> bool {
    let output = std::process::Command::new("lipo")
        .arg("-archs")
        .arg(binary_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).split_whitespace().any(|a| a == arch)
        }
        _ => false,
    }
}

/// Default guess for where the compiled barriers binary lives.
///
/// Lookup order (for the no-AppHandle case):
/// 1. Dev path (cargo run from src-tauri/ → three levels up to repo root).
/// 2. Legacy Barrier.app install.
/// For the production-bundled case, use `resolve_binary(app)` instead, which
/// consults Tauri's resource_dir.
pub fn default_binary_path() -> String {
    // 1. Dev path (cargo run from src-tauri/ → three levels up to repo root).
    let dev_candidates = [
        "../../../build/flowdesk-helper/bin/barriers",
        "../../build/flowdesk-helper/bin/barriers",
        "../../../build/bin/barriers",
        "../../build/bin/barriers",
    ];
    for c in dev_candidates {
        if std::path::Path::new(c).exists() {
            return c.into();
        }
    }

    // 2. Legacy Barrier.app install.
    let app_candidates = [
        "/Applications/FlowDesk.app/Contents/Resources/bin/barriers",
        "/Applications/Barrier.app/Contents/MacOS/barriers",
    ];
    for c in app_candidates {
        if std::path::Path::new(c).exists() {
            return c.into();
        }
    }

    // Last resort: hope barriers is on PATH.
    "barriers".into()
}

/// Resolve the barriers binary using the live AppHandle, which lets us find
/// the bundled resource in production. This is the preferred entry point.
pub fn resolve_binary<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> String {
    use tauri::Manager;

    // Production lookup order (Contents/<dir>/bin/barriers):
    //   1. Helpers/ — macOS-standard location for bundled helper tools.
    //      Does NOT cause a second Dock icon (unlike MacOS/, where any
    //      executable shows up as an app instance).
    //   2. Resources/ — Tauri's default resources dir; works but doesn't
    //      inherit Accessibility on its own.
    //   3. MacOS/ — last resort (causes duplicate Dock icon, avoided).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            // macos_dir = Contents/MacOS; contents_dir = Contents
            if let Some(contents_dir) = macos_dir.parent() {
                for sub in ["Helpers", "Resources"] {
                    let candidate = contents_dir.join(sub).join("bin").join("barriers");
                    if candidate.exists() {
                        return candidate.to_string_lossy().into_owned();
                    }
                }
            }
        }
    }

    // Fallback: Tauri resource_dir() (Contents/Resources on macOS).
    if let Ok(dir) = app.path().resource_dir() {
        let candidate = dir.join("bin").join("barriers");
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    // Dev fallback.
    default_binary_path()
}
