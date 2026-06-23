// Shared TypeScript types mirroring the Rust structs.

export type ServerState = "stopped" | "starting" | "connected" | "disconnected" | "error";

export type Direction = "left" | "right" | "up" | "down";
export type Modifier = "shift" | "ctrl" | "alt" | "meta" | "super";
export type ActionType =
  | "keydown"
  | "keyup"
  | "keystroke"
  | "switchtoscreen"
  | "switchindirection"
  | "lockcursortoscreen"
  | "togglescreen";

export interface Fixes {
  half_duplex_caps_lock: boolean;
  half_duplex_num_lock: boolean;
  half_duplex_scroll_lock: boolean;
  xtest_is_xinerama_unaware: boolean;
  preserve_focus: boolean;
}

export interface Screen {
  name: string;
  aliases: string[];
  modifiers: (string | null)[];
  switch_corners: boolean[];
  switch_corner_size: number;
  fixes: Fixes;
}

export interface KeySequence {
  0: string[];
}

export interface Action {
  type: ActionType;
  target_screen: string | null;
  direction: Direction | null;
  keys: KeySequence;
}

export interface Hotkey {
  keys: KeySequence;
  actions: Action[];
}

export interface ServerConfig {
  num_columns: number;
  num_rows: number;
  screens: Screen[];
  has_heartbeat: boolean;
  heartbeat: number;
  relative_mouse_moves: boolean;
  screen_saver_sync: boolean;
  win32_keep_foreground: boolean;
  has_switch_delay: boolean;
  switch_delay: number;
  has_switch_double_tap: boolean;
  switch_double_tap: number;
  switch_corner_size: number;
  switch_corners: boolean[];
  ignore_auto_config_client: boolean;
  enable_drag_and_drop: boolean;
  clipboard_sharing: boolean;
  hotkeys: Hotkey[];
}

export interface AppConfig {
  screen_name: string;
  port: number;
  interface: string;
  log_level: string;
  log_to_file: boolean;
  log_filename: string;
  language: string;
  crypto_enabled: boolean;
  require_client_certificate: boolean;
  auto_hide: boolean;
  auto_start: boolean;
  minimize_to_tray: boolean;
  enable_drag_and_drop: boolean;
  server_config: ServerConfig;
}

export interface LogLine {
  level: string;
  message: string;
}

export interface FingerprintPrompt {
  sha1: string;
  sha256: string;
}

// Helpers for KeySequence — serde serializes tuple structs oddly; normalize.
export function emptyScreen(): Screen {
  return {
    name: "",
    aliases: [],
    modifiers: [null, null, null, null, null],
    switch_corners: [false, false, false, false],
    switch_corner_size: 0,
    fixes: {
      half_duplex_caps_lock: false,
      half_duplex_num_lock: false,
      half_duplex_scroll_lock: false,
      xtest_is_xinerama_unaware: false,
      preserve_focus: false,
    },
  };
}

export function namedScreen(name: string): Screen {
  return { ...emptyScreen(), name };
}
