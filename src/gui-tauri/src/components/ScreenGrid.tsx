import { useState } from "react";
import type { ServerConfig, Screen } from "../types";
import { emptyScreen, namedScreen } from "../types";

// MIME type used for screen drag/drop payloads (mirrors the legacy
// "application/x-qbarrier-screen" custom MIME).
const DRAG_MIME = "application/x-flowdesk-screen";

/**
 * The screen layout editor: a num_columns × num_rows grid.
 *
 * - Drag the "New screen" palette item onto an empty cell to add.
 * - Drag a filled cell to another cell to move (or swap if occupied).
 * - Drag a filled cell to the trash to delete.
 * - Click a cell to select; double-click to edit; Delete key removes.
 * - Ctrl+Arrow swaps the selected screen with its neighbour.
 *
 * Mirrors the legacy ScreenSetupView/ScreenSetupModel behaviour.
 */
export function ScreenGrid({
  config,
  localScreenName,
  onSave,
}: {
  config: ServerConfig;
  localScreenName: string;
  onSave: (c: ServerConfig) => Promise<void>;
}) {
  const [cfg, setCfg] = useState<ServerConfig>(config);
  const [selected, setSelected] = useState<number | null>(null);
  const [editing, setEditing] = useState<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);
  const [saved, setSaved] = useState(false);

  // Normalize: ensure screens array length matches grid.
  const expected = cfg.num_columns * cfg.num_rows;
  if (cfg.screens.length !== expected) {
    const next = [...cfg.screens];
    while (next.length < expected) next.push(emptyScreen());
    next.length = expected;
    cfg.screens = next;
  }

  const update = (next: ServerConfig) => {
    setCfg(next);
    setSaved(false);
  };

  const setCell = (i: number, screen: Screen) => {
    const screens = [...cfg.screens];
    screens[i] = screen;
    update({ ...cfg, screens });
  };

  const onPaletteDragStart = (e: React.DragEvent) => {
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify({ source: -1 }));
    e.dataTransfer.effectAllowed = "copyMove";
  };

  const onCellDragStart = (i: number) => (e: React.DragEvent) => {
    e.dataTransfer.setData(DRAG_MIME, JSON.stringify({ source: i }));
    e.dataTransfer.effectAllowed = "copyMove";
  };

  const onCellDragOver = (i: number) => (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(i);
  };

  const onCellDrop = (i: number) => (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(null);
    const raw = e.dataTransfer.getData(DRAG_MIME);
    if (!raw) return;
    let payload: { source: number };
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }

    const screens = [...cfg.screens];
    if (payload.source === -1) {
      // Adding a new screen onto cell i (must be empty).
      if (screens[i].name === "") {
        screens[i] = namedScreen("unnamed");
      }
    } else {
      const src = payload.source;
      if (src === i) return;
      // Move or swap.
      if (screens[i].name === "") {
        screens[i] = screens[src];
        screens[src] = emptyScreen();
      } else {
        const tmp = screens[i];
        screens[i] = screens[src];
        screens[src] = tmp;
      }
    }
    update({ ...cfg, screens });
    setSelected(i);
  };

  const onTrashDrop = (e: React.DragEvent) => {
    e.preventDefault();
    const raw = e.dataTransfer.getData(DRAG_MIME);
    if (!raw) return;
    let payload: { source: number };
    try {
      payload = JSON.parse(raw);
    } catch {
      return;
    }
    if (payload.source >= 0) {
      const screens = [...cfg.screens];
      screens[payload.source] = emptyScreen();
      update({ ...cfg, screens });
      setSelected(null);
    }
  };

  const swap = (i: number, dir: "left" | "right" | "up" | "down") => {
    const cols = cfg.num_columns;
    const col = i % cols;
    const row = Math.floor(i / cols);
    const [nc, nr] = {
      left: [col - 1, row],
      right: [col + 1, row],
      up: [col, row - 1],
      down: [col, row + 1],
    }[dir] as [number, number];
    if (nc < 0 || nr < 0 || nc >= cols || nr >= cfg.num_rows) return;
    const j = nr * cols + nc;
    const screens = [...cfg.screens];
    [screens[i], screens[j]] = [screens[j], screens[i]];
    update({ ...cfg, screens });
    setSelected(j);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (selected === null) return;
    if (e.key === "Delete" || e.key === "Backspace") {
      const screens = [...cfg.screens];
      screens[selected] = emptyScreen();
      update({ ...cfg, screens });
      setSelected(null);
      return;
    }
    if (e.ctrlKey || e.metaKey) {
      const map: Record<string, "left" | "right" | "up" | "down"> = {
        ArrowLeft: "left",
        ArrowRight: "right",
        ArrowUp: "up",
        ArrowDown: "down",
      };
      const dir = map[e.key];
      if (dir) {
        e.preventDefault();
        swap(selected, dir);
      }
    }
  };

  const gridStyle = {
    gridTemplateColumns: `repeat(${cfg.num_columns}, 1fr)`,
    gridTemplateRows: `repeat(${cfg.num_rows}, 1fr)`,
  };

  const submit = async () => {
    await onSave(cfg);
    setSaved(true);
  };

  return (
    <div
      className="panel"
      style={{ display: "flex", flexDirection: "column" }}
      tabIndex={0}
      onKeyDown={onKeyDown}
    >
      <div className="palette">
        <div
          className="palette-item"
          draggable
          onDragStart={onPaletteDragStart}
          title="Drag onto an empty cell to add a screen"
        >
          ＋ New screen
        </div>
        <div
          className="palette-item"
          onDragOver={(e) => e.preventDefault()}
          onDrop={onTrashDrop}
          title="Drag a screen here to delete it"
          style={{ color: "var(--danger)" }}
        >
          🗑 Delete
        </div>
        <span style={{ color: "var(--muted)", flex: 1 }}>
          Tip: Ctrl+Arrow to swap, Del to remove, double-click to edit.
        </span>
        <button onClick={submit}>Save layout</button>
        {saved && <span style={{ color: "var(--success)" }}>Saved.</span>}
      </div>

      <div className="grid" style={gridStyle}>
        {cfg.screens.map((s, i) => {
          const isServer = s.name === localScreenName && s.name !== "";
          const cls = [
            "grid-cell",
            s.name ? "filled" : "",
            isServer ? "server" : "",
            dragOver === i ? "drag-over" : "",
            selected === i ? "selected" : "",
          ]
            .filter(Boolean)
            .join(" ");
          return (
            <div
              key={i}
              className={cls}
              draggable={s.name !== ""}
              onDragStart={onCellDragStart(i)}
              onDragOver={onCellDragOver(i)}
              onDrop={onCellDrop(i)}
              onClick={() => setSelected(i)}
              onDoubleClick={() => s.name && setEditing(i)}
              title={isServer ? `${s.name} (this Mac)` : s.name || "(empty)"}
            >
              {s.name || "—"}
              {isServer && <div style={{ fontSize: 9 }}>server</div>}
            </div>
          );
        })}
      </div>

      {editing !== null && cfg.screens[editing]?.name && (
        <ScreenEditDialog
          screen={cfg.screens[editing]}
          onCancel={() => setEditing(null)}
          onSave={(s) => {
            setCell(editing, s);
            setEditing(null);
          }}
        />
      )}
    </div>
  );
}

function ScreenEditDialog({
  screen,
  onSave,
  onCancel,
}: {
  screen: Screen;
  onSave: (s: Screen) => void;
  onCancel: () => void;
}) {
  const [name, setName] = useState(screen.name);
  const [aliases, setAliases] = useState(screen.aliases.join(", "));

  const submit = () => {
    onSave({
      ...screen,
      name: name.trim() || screen.name,
      aliases: aliases
        .split(",")
        .map((a) => a.trim())
        .filter(Boolean),
    });
  };

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
      onClick={onCancel}
    >
      <div
        className="panel"
        style={{ width: 320, background: "var(--panel)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <h3 style={{ margin: "0 0 12px" }}>Edit screen</h3>
        <div className="field">
          <label>Name</label>
          <input value={name} onChange={(e) => setName(e.target.value)} autoFocus />
        </div>
        <div className="field">
          <label>Aliases</label>
          <input
            value={aliases}
            placeholder="comma-separated"
            onChange={(e) => setAliases(e.target.value)}
          />
        </div>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end", marginTop: 12 }}>
          <button className="secondary" onClick={onCancel}>
            Cancel
          </button>
          <button onClick={submit}>Save</button>
        </div>
      </div>
    </div>
  );
}
