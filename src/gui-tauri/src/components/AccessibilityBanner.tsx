import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api";

/**
 * Banner that surfaces when the BARRIERS subprocess can't capture the mouse.
 *
 * Important: the permission macOS cares about is for the BARRIERS binary
 * itself (the CGEventTap caller), not for FlowDesk. We can't query that
 * directly, so we drive this banner off the `permission://needed` event the
 * supervisor emits when barriers logs "cursor may not be visible".
 *
 * Mirrors the legacy main.cpp assistive-devices gate.
 */
export function AccessibilityBanner() {
  const [needed, setNeeded] = useState<string | null>(null);

  useEffect(() => {
    // 1. On mount, also do a best-effort check of the GUI's own permission
    //    (cheap, covers the .app case where GUI == barriers).
    api.checkAccessibility().then((ok) => {
      if (!ok) setNeeded("(flowdesk process)");
    });

    // 2. Listen for the authoritative signal: barriers reporting capture
    //    failure, carrying the exact binary path the user must authorize.
    const unlistenPromise = listen<{ binary_path: string }>(
      "permission://needed",
      (e) => setNeeded(e.payload.binary_path)
    );
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  if (!needed) return null;

  return (
    <div
      style={{
        background: "var(--warning)",
        color: "white",
        padding: "10px 12px",
        borderRadius: 8,
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 12,
      }}
    >
      <span style={{ flex: 1 }}>
        <strong>鼠标捕获失败。</strong>
        请把下面的程序加到辅助功能白名单(系统设置 → 隐私与安全性 → 辅助功能):
        <code
          style={{
            display: "block",
            marginTop: 4,
            background: "rgba(0,0,0,0.2)",
            padding: "2px 6px",
            borderRadius: 4,
            fontFamily: "var(--mono)",
            fontSize: 11,
            userSelect: "all",
          }}
        >
          {needed}
        </code>
      </span>
      <button
        onClick={() =>
          openUrl(
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
          )
        }
        style={{ background: "white", color: "var(--warning)", flexShrink: 0 }}
      >
        打开系统设置
      </button>
    </div>
  );
}
