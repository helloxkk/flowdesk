import { useEffect, useState } from "react";
import * as api from "../api";

/**
 * Banner that appears when macOS Accessibility permission is missing.
 * Mirrors the legacy main.cpp assistive-devices gate. The user must grant
 * permission in System Settings → Privacy & Security → Accessibility, else
 * barriers cannot capture the keyboard/mouse.
 */
export function AccessibilityBanner() {
  const [trusted, setTrusted] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const ok = await api.checkAccessibility();
        if (!cancelled) setTrusted(ok);
      } catch {
        // Non-macOS or unavailable: treat as trusted.
      }
    };
    check();
    // Poll every 3s while not trusted (permission can be granted while the
    // app is open, and the value flips without a restart).
    const interval = setInterval(check, 3000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  if (trusted) return null;

  return (
    <div
      style={{
        background: "var(--warning)",
        color: "white",
        padding: "8px 12px",
        borderRadius: 8,
        display: "flex",
        alignItems: "center",
        gap: 10,
        fontSize: 12,
      }}
    >
      <span style={{ flex: 1 }}>
        FlowDesk needs Accessibility permission to share your keyboard and mouse.
      </span>
      <button
        onClick={() => api.requestAccessibility()}
        style={{ background: "white", color: "var(--warning)" }}
      >
        Open System Settings…
      </button>
    </div>
  );
}
