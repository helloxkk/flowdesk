import { useEffect, useState } from "react";
import * as api from "../api";

/**
 * Permissions guide page.
 *
 * FlowDesk's barriers subprocess needs TWO macOS permissions to work:
 *   1. Accessibility      — keyboard + mouse button capture
 *   2. Screen Recording   — global mouse position (macOS 15+, required for
 *                            mouse motion; without it the cursor stutters)
 *
 * Both are bundle-level grants that macOS caches per-app. Screen Recording
 * requires an app restart to take effect after the user toggles it.
 */
interface PermState {
  accessibility: boolean;
  screenRecording: boolean;
}

export function PermissionGuide() {
  const [perm, setPerm] = useState<PermState>({
    accessibility: true,
    screenRecording: true,
  });
  const [checking, setChecking] = useState(true);

  const refresh = async () => {
    setChecking(true);
    try {
      const [acc, scr] = await Promise.all([
        api.checkAccessibility(),
        api.checkScreenCapture(),
      ]);
      setPerm({ accessibility: acc, screenRecording: scr });
    } catch (e) {
      console.error("permission check failed", e);
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    refresh();
    // Poll every 4s — macOS doesn't push permission-change notifications, and
    // the user toggles these in System Settings while our window is open.
    const interval = setInterval(refresh, 4000);
    return () => clearInterval(interval);
  }, []);

  const allGranted = perm.accessibility && perm.screenRecording;

  return (
    <div className="panel">
      <h3 style={{ margin: "0 0 4px" }}>macOS 权限</h3>
      <p style={{ margin: "0 0 16px", color: "var(--muted)", fontSize: 12 }}>
        FlowDesk 需要以下权限才能共享键盘和鼠标。授权后如果状态没更新,请重启 FlowDesk。
      </p>

      {allGranted && !checking && (
        <div
          style={{
            background: "var(--success)",
            color: "white",
            padding: "10px 12px",
            borderRadius: 8,
            marginBottom: 12,
            fontSize: 12,
          }}
        >
          ✓ 所有权限已就绪。点 Start 开始使用。
        </div>
      )}

      <PermissionRow
        title="辅助功能"
        purpose="捕获键盘和鼠标点击事件"
        granted={perm.accessibility}
        onOpenSettings={() => api.openAccessibilitySettings()}
        onRequest={() => api.requestAccessibility().then(refresh)}
      />

      <PermissionRow
        title="屏幕录制"
        purpose="读取鼠标位置(macOS 15+ 必需,否则鼠标移动会卡顿)"
        granted={perm.screenRecording}
        warnRestart
        onOpenSettings={() => api.openScreenRecordingSettings()}
        onRequest={() => api.requestScreenCapture().then(refresh)}
      />

      <div style={{ marginTop: 16, fontSize: 11, color: "var(--muted)" }}>
        <p style={{ margin: "4px 0" }}>
          • 授权对象是 FlowDesk.app(/Applications/FlowDesk.app)
        </p>
        <p style={{ margin: "4px 0" }}>
          • 屏幕录制权限需要退出并重新打开 FlowDesk 才生效
        </p>
        <p style={{ margin: "4px 0" }}>
          • 如果开关无法打开,先点 − 删除再重新添加 FlowDesk
        </p>
      </div>
    </div>
  );
}

function PermissionRow({
  title,
  purpose,
  granted,
  warnRestart,
  onOpenSettings,
  onRequest,
}: {
  title: string;
  purpose: string;
  granted: boolean;
  warnRestart?: boolean;
  onOpenSettings: () => void;
  onRequest: () => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        padding: "12px",
        background: granted ? "var(--bg)" : "color-mix(in srgb, var(--warning) 10%, var(--bg))",
        border: `1px solid ${granted ? "var(--success)" : "var(--warning)"}`,
        borderRadius: 8,
        marginBottom: 10,
      }}
    >
      <span
        style={{
          fontSize: 20,
          width: 28,
          textAlign: "center",
          flexShrink: 0,
        }}
      >
        {granted ? "✅" : "⚠️"}
      </span>
      <div style={{ flex: 1 }}>
        <div style={{ fontWeight: 600, fontSize: 13 }}>
          {title}
          {granted ? (
            <span style={{ color: "var(--success)", marginLeft: 8, fontWeight: 400 }}>
              已授权
            </span>
          ) : (
            <span style={{ color: "var(--warning)", marginLeft: 8, fontWeight: 400 }}>
              未授权
            </span>
          )}
        </div>
        <div style={{ color: "var(--muted)", fontSize: 11, marginTop: 2 }}>
          {purpose}
          {warnRestart && !granted && (
            <span style={{ color: "var(--warning)" }}> · 授权后需重启 FlowDesk</span>
          )}
        </div>
      </div>
      {!granted && (
        <div style={{ display: "flex", gap: 6, flexShrink: 0 }}>
          <button className="secondary" onClick={onRequest} title="触发系统授权弹窗">
            申请
          </button>
          <button onClick={onOpenSettings} title="打开系统设置对应面板">
            打开设置
          </button>
        </div>
      )}
      {granted && (
        <button
          className="secondary"
          onClick={onOpenSettings}
          style={{ flexShrink: 0 }}
          title="打开系统设置查看详情"
        >
          设置
        </button>
      )}
    </div>
  );
}
