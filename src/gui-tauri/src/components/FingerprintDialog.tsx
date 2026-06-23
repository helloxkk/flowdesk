import type { FingerprintPrompt } from "../types";

/**
 * Modal that pops up when barriers reports a TLS peer fingerprint we haven't
 * trusted yet. Mirrors the legacy FingerprintAcceptDialog. For now we record
 * the trust decision locally (Phase 4 stores it under the config dir); a
 * full implementation would write to the trusted-fingerprints file that the
 * C++ core consults.
 */
export function FingerprintDialog({
  prompt,
  onAccept,
  onReject,
}: {
  prompt: FingerprintPrompt;
  onAccept: () => void;
  onReject: () => void;
}) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.4)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
      onClick={onReject}
    >
      <div
        className="panel"
        style={{ width: 420, background: "var(--panel)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <h3 style={{ margin: "0 0 8px" }}>Trust this server?</h3>
        <p style={{ margin: "0 0 12px", color: "var(--muted)" }}>
          A client is connecting with this TLS fingerprint. Accept it to allow
          the connection.
        </p>
        <div style={{ fontFamily: "var(--mono)", fontSize: 11, marginBottom: 6 }}>
          <div>
            <strong>SHA256:</strong> {prompt.sha256}
          </div>
          <div>
            <strong>SHA1:</strong> {prompt.sha1}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", marginTop: 14 }}>
          <button className="secondary" onClick={onReject}>
            Reject
          </button>
          <button onClick={onAccept}>Trust</button>
        </div>
      </div>
    </div>
  );
}
