import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "./api";
import type { AppConfig, LogLine, ServerConfig, ServerState } from "./types";
import { LogPanel } from "./components/LogPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { ScreenGrid } from "./components/ScreenGrid";

type Tab = "main" | "screens" | "settings";

export default function App() {
  const [state, setState] = useState<ServerState>("stopped");
  const [tab, setTab] = useState<Tab>("main");
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [serverCfg, setServerCfg] = useState<ServerConfig | null>(null);
  const [localIps, setLocalIps] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);

  const refreshStatus = useCallback(async () => {
    try {
      setState(await api.getStatus());
    } catch (e) {
      console.error("getStatus failed", e);
    }
  }, []);

  const loadAll = useCallback(async () => {
    try {
      const [cfg, sc] = await Promise.all([
        api.getAppConfig(),
        api.getServerConfig(),
      ]);
      setConfig(cfg);
      setServerCfg(sc);
    } catch (e) {
      console.error("load config failed", e);
    }
  }, []);

  useEffect(() => {
    refreshStatus();
    loadAll();
    api.getLocalIps().then(setLocalIps).catch(() => {});

    const unsubs: Array<() => void> = [];
    listen<ServerState>("state://change", (e) => setState(e.payload)).then((u) =>
      unsubs.push(u)
    );
    listen<LogLine>("log://line", (e) => {
      setLogs((prev) => {
        const next = [...prev, e.payload];
        // Cap log buffer to avoid runaway memory.
        return next.length > 5000 ? next.slice(-5000) : next;
      });
    }).then((u) => unsubs.push(u));

    return () => {
      unsubs.forEach((u) => u());
    };
  }, [refreshStatus, loadAll]);

  const onStart = async () => {
    setBusy(true);
    try {
      await api.startServer();
    } catch (e) {
      console.error(e);
      setLogs((p) => [
        ...p,
        { level: "ERROR", message: String(e) },
      ]);
    } finally {
      setBusy(false);
      refreshStatus();
    }
  };

  const onStop = async () => {
    setBusy(true);
    try {
      await api.stopServer();
    } catch (e) {
      console.error(e);
    } finally {
      setBusy(false);
      refreshStatus();
    }
  };

  const onSaveAppConfig = async (c: AppConfig) => {
    await api.saveAppConfig(c);
    setConfig(c);
  };

  const onSaveServerConfig = async (c: ServerConfig) => {
    await api.saveServerConfig(c);
    setServerCfg(c);
    // Also reflect into the app config so a restart uses it.
    if (config) {
      const next = { ...config, server_config: c };
      setConfig(next);
    }
  };

  if (!config || !serverCfg) {
    return <div className="app">Loading…</div>;
  }

  const running = state === "starting" || state === "connected";

  return (
    <div className="app">
      <div className="status-bar">
        <span className={`dot ${state}`} />
        <strong style={{ flex: 1 }}>
          {stateLabel(state)}
        </strong>
        {running ? (
          <button className="danger" onClick={onStop} disabled={busy}>
            Stop
          </button>
        ) : (
          <button onClick={onStart} disabled={busy}>
            Start
          </button>
        )}
      </div>

      <div className="tabs">
        <button
          className={`tab ${tab === "main" ? "active" : ""}`}
          onClick={() => setTab("main")}
        >
          Server
        </button>
        <button
          className={`tab ${tab === "screens" ? "active" : ""}`}
          onClick={() => setTab("screens")}
        >
          Screens
        </button>
        <button
          className={`tab ${tab === "settings" ? "active" : ""}`}
          onClick={() => setTab("settings")}
        >
          Settings
        </button>
      </div>

      {tab === "main" && (
        <MainPanel
          config={config}
          localIps={localIps}
          running={running}
        />
      )}
      {tab === "screens" && (
        <ScreenGrid
          config={serverCfg}
          localScreenName={config.screen_name}
          onSave={onSaveServerConfig}
        />
      )}
      {tab === "settings" && (
        <SettingsPanel config={config} onSave={onSaveAppConfig} />
      )}

      <LogPanel lines={logs} />
    </div>
  );
}

function stateLabel(s: ServerState): string {
  switch (s) {
    case "stopped":
      return "Stopped";
    case "starting":
      return "Starting…";
    case "connected":
      return "Barrier is running";
    case "disconnected":
      return "Disconnected (restarting…)";
    case "error":
      return "Error";
  }
}

function MainPanel({
  config,
  localIps,
  running,
}: {
  config: AppConfig;
  localIps: string[];
  running: boolean;
}) {
  return (
    <div className="panel" style={{ maxHeight: 160 }}>
      <div className="field">
        <label>Screen name</label>
        <input value={config.screen_name} readOnly />
      </div>
      <div className="field">
        <label>Listen address</label>
        <span style={{ fontFamily: "var(--mono)" }}>
          {config.interface || "all interfaces"}:{config.port}
        </span>
      </div>
      <div className="field">
        <label>Local IPs</label>
        <span style={{ fontFamily: "var(--mono)", color: "var(--muted)" }}>
          {localIps.join(", ") || "(none detected)"}
        </span>
      </div>
      <div className="field">
        <label>TLS</label>
        <span>{config.crypto_enabled ? "Enabled (default)" : "Disabled"}</span>
      </div>
      {running && (
        <p style={{ margin: "8px 0 0", color: "var(--success)" }}>
          Server is listening. Point clients at one of the IPs above on port{" "}
          {config.port}.
        </p>
      )}
    </div>
  );
}
