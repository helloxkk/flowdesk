import { useState } from "react";
import type { AppConfig } from "../types";

const LOG_LEVELS = ["ERROR", "WARNING", "NOTE", "INFO", "DEBUG", "DEBUG1", "DEBUG2"];

export function SettingsPanel({
  config,
  onSave,
}: {
  config: AppConfig;
  onSave: (c: AppConfig) => Promise<void>;
}) {
  const [draft, setDraft] = useState<AppConfig>(config);
  const [saved, setSaved] = useState(false);

  const update = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => {
    setDraft({ ...draft, [key]: value });
    setSaved(false);
  };

  const submit = async () => {
    await onSave(draft);
    setSaved(true);
  };

  return (
    <div className="panel">
      <div className="field">
        <label>Screen name</label>
        <input
          value={draft.screen_name}
          onChange={(e) => update("screen_name", e.target.value)}
        />
      </div>

      <div className="field">
        <label>Port</label>
        <input
          type="number"
          value={draft.port}
          onChange={(e) => update("port", Number(e.target.value))}
        />
      </div>

      <div className="field">
        <label>Listen interface</label>
        <input
          value={draft.interface}
          placeholder="(empty = all)"
          onChange={(e) => update("interface", e.target.value)}
        />
      </div>

      <div className="field">
        <label>Log level</label>
        <select
          value={draft.log_level}
          onChange={(e) => update("log_level", e.target.value)}
        >
          {LOG_LEVELS.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
      </div>

      <div className="field">
        <label>Enable TLS</label>
        <input
          type="checkbox"
          checked={draft.crypto_enabled}
          onChange={(e) => update("crypto_enabled", e.target.checked)}
        />
      </div>

      <div className="field">
        <label>Require client cert</label>
        <input
          type="checkbox"
          checked={draft.require_client_certificate}
          onChange={(e) =>
            update("require_client_certificate", e.target.checked)
          }
        />
      </div>

      <div className="field">
        <label>Drag &amp; drop</label>
        <input
          type="checkbox"
          checked={draft.enable_drag_and_drop}
          onChange={(e) => update("enable_drag_and_drop", e.target.checked)}
        />
      </div>

      <div className="field">
        <label>Minimize to tray</label>
        <input
          type="checkbox"
          checked={draft.minimize_to_tray}
          onChange={(e) => update("minimize_to_tray", e.target.checked)}
        />
      </div>

      <div className="field">
        <label>Start at login</label>
        <input
          type="checkbox"
          checked={draft.auto_start}
          onChange={(e) => update("auto_start", e.target.checked)}
        />
      </div>

      <div style={{ marginTop: 12, display: "flex", gap: 8, alignItems: "center" }}>
        <button onClick={submit}>Save</button>
        {saved && <span style={{ color: "var(--success)" }}>Saved.</span>}
      </div>
    </div>
  );
}
