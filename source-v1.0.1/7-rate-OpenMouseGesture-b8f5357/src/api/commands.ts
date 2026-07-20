import { invoke } from "@tauri-apps/api/core";
import type { GestureTemplate, Action, Config } from "../types";

export async function getGestures(): Promise<GestureTemplate[]> {
  return invoke<GestureTemplate[]>("get_gestures");
}

export async function saveGesture(name: string, points: [number, number][]): Promise<void> {
  return invoke("save_gesture", { name, points });
}

export async function updateGesture(
  oldName: string,
  newName: string,
  points: [number, number][]
): Promise<void> {
  return invoke("update_gesture", { oldName, newName, points });
}

export async function deleteGesture(name: string): Promise<void> {
  return invoke("delete_gesture", { name });
}

export async function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export async function saveConfig(config: Config): Promise<void> {
  return invoke("save_config", { config });
}

export async function getActions(): Promise<Action[]> {
  return invoke<Action[]>("get_actions");
}

export async function addAction(action: Action): Promise<void> {
  return invoke("add_action", { action });
}

export async function updateAction(actionKey: string, action: Action): Promise<void> {
  return invoke("update_action", { actionKey, action });
}

export async function deleteAction(actionKey: string): Promise<void> {
  return invoke("delete_action", { actionKey });
}

export async function setGestureEnabled(enabled: boolean): Promise<void> {
  return invoke("set_gesture_enabled", { enabled });
}

export async function isGestureEnabled(): Promise<boolean> {
  return invoke<boolean>("is_gesture_enabled");
}

export async function getAutostartStatus(): Promise<boolean> {
  return invoke<boolean>("get_autostart_status");
}

export async function setAutostartEnabled(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_autostart_enabled", { enabled });
}

export async function getConfigFilePath(): Promise<string> {
  return invoke<string>("get_config_file_path");
}

export async function getGesturesFilePath(): Promise<string> {
  return invoke<string>("get_gestures_file_path");
}

export async function resetConfigToDefault(): Promise<void> {
  return invoke("reset_config_to_default");
}

export async function resetGesturesToDefault(): Promise<void> {
  return invoke("reset_gestures_to_default");
}

export async function validateConfigFile(): Promise<boolean> {
  return invoke<boolean>("validate_config_file");
}

export async function validateGesturesFile(): Promise<boolean> {
  return invoke<boolean>("validate_gestures_file");
}

export async function getConfigValidationError(): Promise<string | null> {
  return invoke<string | null>("get_config_validation_error");
}

export async function getGesturesValidationError(): Promise<string | null> {
  return invoke<string | null>("get_gestures_validation_error");
}

export async function getVersion(): Promise<string> {
  return invoke<string>("get_version");
}

export async function getIconBytes(): Promise<number[]> {
  return invoke<number[]>("get_icon_bytes");
}

export async function exportSettingsBundle(path: string): Promise<void> {
  return invoke("export_settings_bundle", { path });
}

export async function importSettingsBundle(path: string): Promise<void> {
  return invoke("import_settings_bundle", { path });
}
