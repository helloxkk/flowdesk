// Thin Tauri invoke wrappers, typed against the Rust command layer.
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  ServerConfig,
  ServerState,
} from "./types";

export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

export async function getStatus(): Promise<ServerState> {
  return invoke<ServerState>("get_status");
}

export async function startServer(): Promise<void> {
  await invoke("start_server");
}

export async function stopServer(): Promise<void> {
  await invoke("stop_server");
}

export async function getAppConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_app_config");
}

export async function saveAppConfig(config: AppConfig): Promise<void> {
  await invoke("save_app_config", { config });
}

export async function getServerConfig(): Promise<ServerConfig> {
  return invoke<ServerConfig>("get_server_config");
}

export async function saveServerConfig(config: ServerConfig): Promise<void> {
  await invoke("save_server_config", { config });
}

export async function getLocalIps(): Promise<string[]> {
  return invoke<string[]>("get_local_ips");
}

export async function checkAccessibility(): Promise<boolean> {
  return invoke<boolean>("check_accessibility");
}

export async function requestAccessibility(): Promise<boolean> {
  return invoke<boolean>("request_accessibility");
}

export async function openAccessibilitySettings(): Promise<void> {
  await invoke("open_accessibility_settings");
}
