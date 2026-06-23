import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import * as api from "../api";

/**
 * Permission banner. On macOS, barriers needs TWO permissions to capture
 * mouse motion:
 *
 *  1. Accessibility  — for keyboard/input injection (the classic barrier need).
 *  2. Screen Recording — required since macOS 15 (Sequoia) for reading the
 *     global mouse position via CGEventGetLocation. Without it, barriers can
 *     capture keyboard + clicks but NOT mouse motion — which reads as
 *     "slow / drops frames" to the user.
 *
 * We surface both independently: Accessibility from the barriers
 * `permission://needed` event (the supervisor emits it when barriers logs
 * "cursor may not be visible"), and Screen Recording from a direct preflight
 * check + the same event as a fallback reminder.
 */
type Missing = "accessibility" | "screenRecording" | null;

export function AccessibilityBanner() {
  const [missing, setMissing] = useState<Missing>(null);

  useEffect(() => {
    // 1. Best-effort screen-recording preflight on mount. macOS 15+ needs it.
    api.checkScreenCapture().then((ok) => {
      if (!ok) setMissing("screenRecording");
    });

    // 2. Listen for the authoritative accessibility-missing signal from the
    //    supervisor (barriers reported cursor capture failure).
    const unlistenPromise = listen<{ binary_path: string }>(
      "permission://needed",
      () => {
        // Accessibility is the more common cause; show that first, but if
        // screen recording is also missing it'll be picked up by the preflight.
        setMissing((cur) => (cur === "screenRecording" ? cur : "accessibility"));
      }
    );
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  if (!missing) return null;

  if (missing === "screenRecording") {
    return (
      <Banner
        color="var(--danger)"
        title="屏幕录制权限缺失(鼠标移动必需)"
        body={
          "macOS 15+ 要求屏幕录制权限才能读取鼠标位置。请授权后重启 FlowDesk,否则鼠标跨屏会卡顿/掉帧。"
        }
        buttonLabel="打开屏幕录制设置"
        onButtonClick={() => api.openScreenRecordingSettings()}
      />
    );
  }

  return (
    <Banner
      color="var(--warning)"
      title="辅助功能权限缺失(键鼠捕获必需)"
      body={
        "barriers 进程需要辅助功能权限才能捕获键盘和鼠标。请把 FlowDesk 加入白名单。"
      }
      buttonLabel="打开辅助功能设置"
      onButtonClick={() => api.openAccessibilitySettings()}
    />
  );
}

function Banner({
  color,
  title,
  body,
  buttonLabel,
  onButtonClick,
}: {
  color: string;
  title: string;
  body: string;
  buttonLabel: string;
  onButtonClick: () => void;
}) {
  return (
    <div
      style={{
        background: color,
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
        <strong>{title}</strong>
        <br />
        {body}
      </span>
      <button
        onClick={onButtonClick}
        style={{ background: "white", color, flexShrink: 0 }}
      >
        {buttonLabel}
      </button>
    </div>
  );
}
