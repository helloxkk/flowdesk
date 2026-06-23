import { useEffect, useRef } from "react";
import type { LogLine } from "../types";

export function LogPanel({ lines }: { lines: LogLine[] }) {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [lines]);

  return (
    <div className="log">
      {lines.length === 0 && (
        <div style={{ color: "var(--muted)" }}>No log output yet.</div>
      )}
      {lines.map((l, i) => (
        <div key={i} className={`log-line ${l.level}`}>
          {l.message}
        </div>
      ))}
      <div ref={endRef} />
    </div>
  );
}
