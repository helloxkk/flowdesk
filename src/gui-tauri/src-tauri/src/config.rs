// FlowDesk server configuration model + barrier text-format serializer.
//
// Mirrors the legacy `ServerConfig` / `Screen` / `Hotkey` structs and the
// `operator<<(QTextStream&, const ServerConfig&)` serializer in
// src/gui/src/ServerConfig.cpp:211-282. The grid is `num_columns * num_rows`
// cells (default 5×3), empty cells hold a `Screen` with an empty name, and
// screen adjacency is derived from grid position (see §3.2 of the design doc).
//
// Copyright (C) 2026 helloxkk (FlowDesk)
// Licensed under GPLv2.

use serde::{Deserialize, Serialize};

pub const DEFAULT_NUM_COLUMNS: u32 = 5;
pub const DEFAULT_NUM_ROWS: u32 = 3;
pub const SERVER_DEFAULT_INDEX: usize = 7; // center cell of 5×3

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn all() -> [Direction; 4] {
        [Direction::Left, Direction::Right, Direction::Up, Direction::Down]
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }

    /// Offset (dcol, drow) on the screen grid for this direction.
    pub fn offset(self) -> (i32, i32) {
        match self {
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
        }
    }
}

/// Modifiers that can be remapped per screen (Shift/Ctrl/Alt/Meta/Super).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    Shift,
    Ctrl,
    Alt,
    Meta,
    Super,
}

impl Modifier {
    pub fn all() -> [Modifier; 5] {
        [Modifier::Shift, Modifier::Ctrl, Modifier::Alt, Modifier::Meta, Modifier::Super]
    }

    pub fn name(self) -> &'static str {
        match self {
            Modifier::Shift => "shift",
            Modifier::Ctrl => "ctrl",
            Modifier::Alt => "alt",
            Modifier::Meta => "meta",
            Modifier::Super => "super",
        }
    }
}

/// Switch corners (the edges of the grid that don't trigger screen switch).
/// Order: TopLeft, TopRight, BottomLeft, BottomRight.
pub const NUM_SWITCH_CORNERS: usize = 4;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Fixes {
    pub half_duplex_caps_lock: bool,
    pub half_duplex_num_lock: bool,
    pub half_duplex_scroll_lock: bool,
    pub xtest_is_xinerama_unaware: bool,
    pub preserve_focus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// The screen name (matches the `--name` the peer reports). Empty = empty cell.
    pub name: String,
    pub aliases: Vec<String>,
    /// Per-modifier remap target, if any. Index by Modifier::all().
    pub modifiers: [Option<String>; 5],
    /// Per-corner switch-corner flag.
    pub switch_corners: [bool; NUM_SWITCH_CORNERS],
    pub switch_corner_size: u32,
    pub fixes: Fixes,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            name: String::new(),
            aliases: Vec::new(),
            modifiers: Default::default(),
            switch_corners: [false; NUM_SWITCH_CORNERS],
            switch_corner_size: 0,
            fixes: Fixes::default(),
        }
    }
}

impl Screen {
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    pub fn named(name: impl Into<String>) -> Self {
        let mut s = Self::default();
        s.name = name.into();
        s
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeySequence(pub Vec<String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    KeyDown,
    KeyUp,
    Keystroke,
    SwitchToScreen,
    SwitchInDirection,
    LockCursorToScreen,
    ToggleScreen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub target_screen: Option<String>,
    pub direction: Option<Direction>,
    pub keys: KeySequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotkey {
    pub keys: KeySequence,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub num_columns: u32,
    pub num_rows: u32,
    /// Length is always num_columns * num_rows. Empty cells have empty names.
    pub screens: Vec<Screen>,
    pub has_heartbeat: bool,
    pub heartbeat: u32,
    pub relative_mouse_moves: bool,
    pub screen_saver_sync: bool,
    pub win32_keep_foreground: bool,
    pub has_switch_delay: bool,
    pub switch_delay: u32,
    pub has_switch_double_tap: bool,
    pub switch_double_tap: u32,
    pub switch_corner_size: u32,
    pub switch_corners: [bool; NUM_SWITCH_CORNERS],
    pub ignore_auto_config_client: bool,
    pub enable_drag_and_drop: bool,
    pub clipboard_sharing: bool,
    pub hotkeys: Vec<Hotkey>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let total = (DEFAULT_NUM_COLUMNS * DEFAULT_NUM_ROWS) as usize;
        let mut screens = vec![Screen::default(); total];
        // Place the local server in the center cell by default.
        let local = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "flowdesk".into());
        screens[SERVER_DEFAULT_INDEX] = Screen::named(local);
        Self {
            num_columns: DEFAULT_NUM_COLUMNS,
            num_rows: DEFAULT_NUM_ROWS,
            screens,
            has_heartbeat: false,
            heartbeat: 0,
            relative_mouse_moves: false,
            screen_saver_sync: true,
            win32_keep_foreground: true,
            has_switch_delay: false,
            switch_delay: 0,
            has_switch_double_tap: false,
            switch_double_tap: 0,
            switch_corner_size: 0,
            switch_corners: [false; NUM_SWITCH_CORNERS],
            ignore_auto_config_client: false,
            enable_drag_and_drop: true,
            clipboard_sharing: true,
            hotkeys: Vec::new(),
        }
    }
}

impl ServerConfig {
    /// Ensure `screens` length matches `num_columns * num_rows`.
    pub fn reconcile(&mut self) {
        let expected = (self.num_columns as usize) * (self.num_rows as usize);
        if self.screens.len() != expected {
            self.screens.resize(expected, Screen::default());
        }
    }

    fn idx(&self, col: i32, row: i32) -> Option<usize> {
        if col < 0 || row < 0 {
            return None;
        }
        let c = col as u32;
        let r = row as u32;
        if c >= self.num_columns || r >= self.num_rows {
            return None;
        }
        Some((r * self.num_columns + c) as usize)
    }

    fn cell(&self, col: i32, row: i32) -> Option<&Screen> {
        self.idx(col, row).map(|i| &self.screens[i])
    }

    /// Name of the screen adjacent to `index` in `direction`, if any and non-empty.
    fn neighbor_name(&self, index: usize, direction: Direction) -> Option<String> {
        let cols = self.num_columns as i32;
        let col = (index as i32) % cols;
        let row = (index as i32) / cols;
        let (dc, dr) = direction.offset();
        self.cell(col + dc, row + dr)
            .filter(|s| !s.is_empty())
            .map(|s| s.name.clone())
    }

    /// Serialize to the barrier text config format (consumed by `barriers -c`).
    pub fn to_barrier_config(&self) -> String {
        let mut out = String::new();
        out.push_str("# FlowDesk server configuration\n");

        // section: screens
        out.push_str("section: screens\n");
        for s in &self.screens {
            if s.is_empty() {
                continue;
            }
            out.push_str(&format!("    {}:\n", s.name));
            for (i, m) in Modifier::all().iter().enumerate() {
                if let Some(target) = &s.modifiers[i] {
                    out.push_str(&format!("        {} = {}\n", m.name(), target));
                }
            }
            if s.fixes.half_duplex_caps_lock {
                out.push_str("        halfDuplexCapsLock = true\n");
            }
            if s.fixes.half_duplex_num_lock {
                out.push_str("        halfDuplexNumLock = true\n");
            }
            if s.fixes.half_duplex_scroll_lock {
                out.push_str("        halfDuplexScrollLock = true\n");
            }
            if s.fixes.xtest_is_xinerama_unaware {
                out.push_str("        xtestIsXineramaUnaware = true\n");
            }
            if s.fixes.preserve_focus {
                out.push_str("        preserveFocus = true\n");
            }
            if s.switch_corners.iter().any(|&b| b) {
                let corners = corner_names(&s.switch_corners);
                out.push_str(&format!("        switchCorners = {}\n", corners));
            }
            if s.switch_corner_size > 0 {
                out.push_str(&format!("        switchCornerSize = {}\n", s.switch_corner_size));
            }
        }
        out.push_str("end\n\n");

        // section: aliases
        let any_alias = self.screens.iter().any(|s| !s.is_empty() && !s.aliases.is_empty());
        if any_alias {
            out.push_str("section: aliases\n");
            for s in &self.screens {
                if s.is_empty() || s.aliases.is_empty() {
                    continue;
                }
                out.push_str(&format!("    {}:\n", s.name));
                for a in &s.aliases {
                    out.push_str(&format!("        {}\n", a));
                }
            }
            out.push_str("end\n\n");
        }

        // section: links (derived from grid adjacency)
        out.push_str("section: links\n");
        for (i, s) in self.screens.iter().enumerate() {
            if s.is_empty() {
                continue;
            }
            let mut links: Vec<(Direction, String)> = Direction::all()
                .iter()
                .filter_map(|&d| self.neighbor_name(i, d).map(|n| (d, n)))
                .collect();
            if links.is_empty() {
                continue;
            }
            out.push_str(&format!("    {}:\n", s.name));
            links.sort_by_key(|(d, _)| d.name());
            for (d, n) in links {
                out.push_str(&format!("        {} = {}\n", d.name(), n));
            }
        }
        out.push_str("end\n\n");

        // section: options
        out.push_str("section: options\n");
        if self.has_heartbeat {
            out.push_str(&format!("    heartbeat = {}\n", self.heartbeat));
        }
        if self.relative_mouse_moves {
            out.push_str("    relativeMouseMoves = true\n");
        }
        if self.screen_saver_sync {
            out.push_str("    screenSaverSync = true\n");
        }
        if self.win32_keep_foreground {
            out.push_str("    win32KeepForeground = true\n");
        }
        out.push_str(&format!("    clipboardSharing = {}\n", self.clipboard_sharing));
        if self.has_switch_delay {
            out.push_str(&format!("    switchDelay = {}\n", self.switch_delay));
        }
        if self.has_switch_double_tap {
            out.push_str(&format!("    switchDoubleTap = {}\n", self.switch_double_tap));
        }
        if self.switch_corners.iter().any(|&b| b) {
            out.push_str(&format!("    switchCorners = {}\n", corner_names(&self.switch_corners)));
        }
        if self.switch_corner_size > 0 {
            out.push_str(&format!("    switchCornerSize = {}\n", self.switch_corner_size));
        }
        // Hotkeys would emit as input-filter rules here (Phase 4).
        out.push_str("end\n");

        out
    }
}

fn corner_names(flags: &[bool; NUM_SWITCH_CORNERS]) -> String {
    const NAMES: [&str; NUM_SWITCH_CORNERS] = ["TopLeft", "TopRight", "BottomLeft", "BottomRight"];
    let mut parts = vec!["none".to_string()];
    for (i, on) in flags.iter().enumerate() {
        if *on {
            parts.push(NAMES[i].to_string());
        }
    }
    parts.join(" + ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let cfg = ServerConfig::default();
        let text = cfg.to_barrier_config();
        assert!(text.contains("section: screens"));
        assert!(text.contains("section: links"));
        assert!(text.contains("section: options"));
    }

    #[test]
    fn neighbor_links_are_derived() {
        let mut cfg = ServerConfig::default();
        // Clear the default server cell so the layout is deterministic.
        for s in &mut cfg.screens {
            s.name.clear();
        }
        // Place two named screens side by side at indexes 6 and 7.
        cfg.screens[6] = Screen::named("alpha");
        cfg.screens[7] = Screen::named("beta");
        // In a 5-wide grid: index 6 = col 1, index 7 = col 2.
        // So alpha's right neighbour is beta, and beta's left neighbour is alpha.
        let text = cfg.to_barrier_config();
        // alpha's right link points to beta.
        assert!(text.contains("right = beta"), "missing alpha→beta link");
        // beta's left link points to alpha.
        assert!(text.contains("left = alpha"), "missing beta→alpha link");
    }

    #[test]
    fn reconcile_resizes_screens() {
        let mut cfg = ServerConfig::default();
        cfg.num_columns = 4;
        cfg.num_rows = 2;
        cfg.reconcile();
        assert_eq!(cfg.screens.len(), 8);
    }
}
