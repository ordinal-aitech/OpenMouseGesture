mod action_label_overlay;
mod command_executor;
mod config;
mod gesture_recognizer;
mod mouse_hook;
mod trajectory_renderer;

use config::{Action, Config, ConfigManager};
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::fs;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager, WebviewWindow,
};

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CAPTION_COLOR},
};

static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
static GESTURE_ENABLED: Mutex<bool> = Mutex::new(true);
static TRAJECTORY_ENABLED: Mutex<bool> = Mutex::new(true);
static TRAY_ICON: OnceCell<Mutex<Option<TrayIcon>>> = OnceCell::new();
static TOGGLE_MENU_ITEM: OnceCell<MenuItem<tauri::Wry>> = OnceCell::new();
const ACTION_LABEL_OVERLAY_ENABLED: bool = false;

#[derive(Clone, Serialize)]
pub struct GestureEvent {
    pub name: String,
    pub action_type: Option<String>,
}

pub fn normalize_trigger_slot(slot: &str) -> &'static str {
    match slot {
        "B" => "B",
        "C" => "C",
        _ => "A",
    }
}

pub fn action_key_for_action(action: &Action) -> String {
    if action.trigger_type == "wheel" {
        return format!(
            "wheel:{}",
            action.wheel_trigger.as_deref().unwrap_or_default()
        );
    }

    format!(
        "gesture:{}:{}",
        normalize_trigger_slot(&action.trigger_slot),
        action.gesture
    )
}

pub fn find_action_for_gesture<'a>(
    config: &'a Config,
    trigger_slot: &str,
    gesture_name: &str,
) -> Option<&'a Action> {
    let slot = normalize_trigger_slot(trigger_slot);
    config.actions.iter().find(|a| {
        (a.trigger_type.is_empty() || a.trigger_type == "gesture")
            && normalize_trigger_slot(&a.trigger_slot) == slot
            && a.gesture == gesture_name
    })
}

pub fn color_for_trigger_slot<'a>(config: &'a Config, trigger_slot: &str) -> &'a str {
    match normalize_trigger_slot(trigger_slot) {
        "B" => &config.triggerBColor,
        "C" => &config.triggerCColor,
        _ => &config.triggerAColor,
    }
}

pub fn trigger_button_for_slot<'a>(config: &'a Config, trigger_slot: &str) -> &'a str {
    match normalize_trigger_slot(trigger_slot) {
        "B" => &config.triggerB,
        "C" => &config.triggerC,
        _ => &config.triggerA,
    }
}

pub fn is_gesture_enabled_internal() -> bool {
    *GESTURE_ENABLED.lock().unwrap()
}

pub fn is_trajectory_enabled_internal() -> bool {
    *TRAJECTORY_ENABLED.lock().unwrap()
}

pub fn set_trajectory_enabled_internal(enabled: bool) {
    let mut trajectory_enabled = TRAJECTORY_ENABLED.lock().unwrap();
    *trajectory_enabled = enabled;
}

pub fn set_active_trail_color(hex_color: &str) {
    trajectory_renderer::set_active_color(hex_color);
}

fn load_icon_from_bytes(bytes: &[u8]) -> Option<Image<'static>> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba_data = rgba.into_raw();
    Some(Image::new_owned(rgba_data, width, height))
}

fn update_tray_icon(enabled: bool) {
    if let Some(tray_mutex) = TRAY_ICON.get() {
        if let Ok(mut tray_opt) = tray_mutex.lock() {
            if let Some(tray) = tray_opt.as_mut() {
                let icon_bytes: &[u8] = if enabled {
                    include_bytes!("../icons/128x128.png")
                } else {
                    include_bytes!("../icons/256x256_disabled.png")
                };

                if let Some(icon) = load_icon_from_bytes(icon_bytes) {
                    let _ = tray.set_icon(Some(icon));
                }
            }
        }
    }
}

pub fn emit_gesture_recognized(name: &str, action_type: Option<&str>) {
    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(
            "gesture-recognized",
            GestureEvent {
                name: name.to_string(),
                action_type: action_type.map(|s| s.to_string()),
            },
        );
    }
}

pub fn emit_trajectory_update(points: &[(i32, i32)], is_drawing: bool) {
    if !is_trajectory_enabled_internal() {
        return;
    }

    // mouse_hook のフックスレッドから同期呼び出しする。以前はここを
    // std::thread::spawn していたため、begin_gesture の「1点だけの状態に
    // 置き換える」呼び出しが、その直後に届く WM_MOUSEMOVE 由来の
    // append_trajectory_point(同期呼び出し)より後にスケジュールされることが
    // あった。その場合、既に追記された複数点が spawn されたスレッドの
    // clear+extend で丸ごと消え、軌跡が一瞬縮んでから再び伸びるように見える
    // (=軌跡全体が追いかけてくるように震える)。呼び出し元の発生順どおりに
    // 状態が更新されることを保証するため、この呼び出しは同期のまま行う。
    trajectory_renderer::update_trajectory(points, is_drawing);
}

pub fn append_trajectory_point(x: i32, y: i32) {
    if !is_trajectory_enabled_internal() {
        return;
    }

    trajectory_renderer::append_trajectory_point(x, y);
}

pub fn clear_trajectory_display() {
    trajectory_renderer::clear_trajectory_display();
}

fn action_type_label(action_type: &str) -> &'static str {
    match action_type {
        "keystroke" => "ホットキー",
        "command" => "コマンド",
        "url" => "URL",
        "window_operation" => "ウィンドウ操作",
        _ => "アクション",
    }
}

fn action_detail(action: &Action) -> Option<String> {
    match action.action_type.as_str() {
        "keystroke" => {
            let key = action.keystroke.as_ref()?.trim();
            if key.is_empty() {
                return None;
            }

            let mut keys = Vec::new();
            if let Some(modifiers) = &action.modifiers {
                for modifier in modifiers {
                    let modifier = modifier.trim();
                    if !modifier.is_empty() {
                        keys.push(modifier.to_string());
                    }
                }
            }
            keys.push(key.to_string());
            Some(keys.join("+"))
        }
        "command" => action
            .command
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        "url" => action
            .url
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        "window_operation" => match action.operation.as_deref() {
            Some("minimize") => Some("最小化".to_string()),
            Some("maximize") => Some("最大化".to_string()),
            Some("close") => Some("閉じる".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn action_label_lines(action: &Action) -> (String, Option<String>) {
    let detail = action_detail(action);
    let name = action.name.trim();

    if !name.is_empty() {
        let secondary = detail.filter(|d| d != name);
        return (name.to_string(), secondary);
    }

    (action_type_label(&action.action_type).to_string(), detail)
}

pub fn show_action_label_for_action(action: &Action) {
    if !ACTION_LABEL_OVERLAY_ENABLED {
        let _ = action;
        return;
    }
    let (primary, secondary) = action_label_lines(action);
    action_label_overlay::update_action_label(Some(&primary), secondary.as_deref());
}

pub fn clear_action_label_overlay() {
    if !ACTION_LABEL_OVERLAY_ENABLED {
        return;
    }
    action_label_overlay::clear_action_label();
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn is_matching_action_key(action: &Action, action_key: &str) -> bool {
    action_key_for_action(action) == action_key
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_gestures() -> Result<Vec<config::GestureTemplate>, String> {
    let manager = ConfigManager::new()?;
    manager.load_gestures()
}

#[tauri::command]
fn save_gesture(name: String, points: Vec<(f64, f64)>) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut gestures = manager.load_gestures()?;

    if gestures.iter().any(|g| g.name == name) {
        return Err(format!("Gesture '{}' already exists", name));
    }

    gestures.push(config::GestureTemplate { name, points });
    manager.save_gestures(&gestures)?;
    Ok(())
}

#[tauri::command]
fn update_gesture(oldName: String, newName: String, points: Vec<(f64, f64)>) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut gestures = manager.load_gestures()?;

    if oldName != newName && gestures.iter().any(|g| g.name == newName) {
        return Err(format!("Gesture '{}' already exists", newName));
    }

    if let Some(gesture) = gestures.iter_mut().find(|g| g.name == oldName) {
        gesture.name = newName;
        gesture.points = points;
        manager.save_gestures(&gestures)?;
        Ok(())
    } else {
        Err(format!("Gesture '{}' not found", oldName))
    }
}

#[tauri::command]
fn delete_gesture(name: String) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut gestures = manager.load_gestures()?;
    gestures.retain(|g| g.name != name);
    manager.save_gestures(&gestures)?;
    Ok(())
}

#[tauri::command]
fn get_license_info() -> Result<String, String> {
    Ok(include_str!("../license.html").to_string())
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn get_icon_bytes() -> Vec<u8> {
    include_bytes!("../icons/128x128@2x.png").to_vec()
}

#[tauri::command]
fn export_settings_bundle(path: String) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let bundle = manager.build_settings_bundle()?;
    let json = serde_json::to_string_pretty(&bundle)
        .map_err(|e| format!("Failed to serialize settings bundle: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write settings bundle: {}", e))?;
    Ok(())
}

#[tauri::command]
fn import_settings_bundle(path: String) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read settings bundle: {}", e))?;
    let bundle: config::SettingsBundle = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings bundle: {}", e))?;
    manager.import_settings_bundle(bundle)?;
    Ok(())
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    let manager = ConfigManager::new()?;
    manager.load_config()
}

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    set_trajectory_enabled_internal(config.trajectory);
    manager.save_config(&config)
}

#[tauri::command]
fn get_actions() -> Result<Vec<Action>, String> {
    let manager = ConfigManager::new()?;
    let config = manager.load_config()?;
    Ok(config.actions)
}

#[tauri::command]
fn add_action(action: Action) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut config = manager.load_config()?;

    let duplicate = config.actions.iter().any(|a| {
        if action.trigger_type == "wheel" && a.trigger_type == "wheel" {
            a.wheel_trigger == action.wheel_trigger
        } else if action.trigger_type == "gesture" && a.trigger_type == "gesture" {
            normalize_trigger_slot(&a.trigger_slot) == normalize_trigger_slot(&action.trigger_slot)
                && a.gesture == action.gesture
        } else {
            false
        }
    });

    if duplicate {
        if action.trigger_type == "wheel" {
            return Err(format!(
                "Action for wheel trigger '{:?}' already exists",
                action.wheel_trigger
            ));
        } else {
            return Err(format!(
                "Action for trigger slot '{}' and gesture '{}' already exists",
                normalize_trigger_slot(&action.trigger_slot),
                action.gesture
            ));
        }
    }

    config.actions.push(action);
    manager.save_config(&config)?;
    Ok(())
}

#[tauri::command]
fn update_action(actionKey: String, action: Action) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut config = manager.load_config()?;

    if let Some(existing) = config
        .actions
        .iter_mut()
        .find(|a| is_matching_action_key(a, &actionKey))
    {
        *existing = action;
        manager.save_config(&config)?;
        Ok(())
    } else {
        Err(format!("Action '{}' not found", actionKey))
    }
}

#[tauri::command]
fn delete_action(actionKey: String) -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let mut config = manager.load_config()?;
    config
        .actions
        .retain(|a| !is_matching_action_key(a, &actionKey));
    manager.save_config(&config)?;
    Ok(())
}

#[tauri::command]
fn set_gesture_enabled(enabled: bool) {
    let mut gesture_enabled = GESTURE_ENABLED.lock().unwrap();
    *gesture_enabled = enabled;
}

#[tauri::command]
fn is_gesture_enabled() -> bool {
    *GESTURE_ENABLED.lock().unwrap()
}

#[tauri::command]
fn get_config_file_path() -> Result<String, String> {
    let manager = ConfigManager::new()?;
    Ok(manager
        .config_dir()
        .join("config.json")
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn get_gestures_file_path() -> Result<String, String> {
    let manager = ConfigManager::new()?;
    Ok(manager
        .config_dir()
        .join("gestures.json")
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn reset_config_to_default() -> Result<(), String> {
    let manager = ConfigManager::new()?;
    // デフォルト上書きは既存のカスタムアクションを破壊し得るため、
    // 上書き前に必ずバックアップを取得する（復元不能な喪失を防ぐ）。
    let backup_dir = manager.backup_before_destructive_write()?;
    eprintln!(
        "[config] backed up config/gestures before reset-to-default: backup={}",
        backup_dir.display()
    );
    let default_config = include_str!("../../config/default-config.json");
    let config: Config = serde_json::from_str(default_config)
        .map_err(|e| format!("Failed to parse default config: {}", e))?;
    manager.save_config(&config)?;
    set_trajectory_enabled_internal(config.trajectory);
    Ok(())
}

#[tauri::command]
fn reset_gestures_to_default() -> Result<(), String> {
    let manager = ConfigManager::new()?;
    let backup_dir = manager.backup_before_destructive_write()?;
    eprintln!(
        "[config] backed up config/gestures before reset-to-default: backup={}",
        backup_dir.display()
    );
    let default_gestures = include_str!("../../config/default-gestures.json");
    let gestures: Vec<config::GestureTemplate> = serde_json::from_str(default_gestures)
        .map_err(|e| format!("Failed to parse default gestures: {}", e))?;
    manager.save_gestures(&gestures)?;
    Ok(())
}

#[tauri::command]
fn validate_config_file() -> Result<bool, String> {
    let manager = ConfigManager::new()?;
    Ok(manager.load_config().is_ok())
}

#[tauri::command]
fn validate_gestures_file() -> Result<bool, String> {
    let manager = ConfigManager::new()?;
    Ok(manager.load_gestures().is_ok())
}

#[tauri::command]
fn get_config_validation_error() -> Result<Option<String>, String> {
    let manager = ConfigManager::new()?;
    match manager.load_config() {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e)),
    }
}

#[tauri::command]
fn get_gestures_validation_error() -> Result<Option<String>, String> {
    let manager = ConfigManager::new()?;
    match manager.load_gestures() {
        Ok(_) => Ok(None),
        Err(e) => Ok(Some(e)),
    }
}

fn toggle_menu_label(enabled: bool) -> &'static str {
    if enabled {
        "ジェスチャーを無効化"
    } else {
        "ジェスチャーを有効化"
    }
}

/// トレイメニューの明示的な項目からのみ呼び出す。単純な左クリックでは
/// 呼ばれない（誤操作でフック全体が無音停止する不具合を防ぐため）。
fn toggle_gesture_enabled(_app: &AppHandle) {
    let mut enabled = GESTURE_ENABLED.lock().unwrap();
    *enabled = !*enabled;
    let new_state = *enabled;
    drop(enabled);

    if new_state {
        let result = mouse_hook::install_hook();
        eprintln!("[tray] gesture toggle -> enabled, hook install result: {:?}", result.is_ok());
    } else {
        let result = mouse_hook::uninstall_hook();
        eprintln!("[tray] gesture toggle -> disabled, hook uninstall result: {:?}", result.is_ok());
    }

    update_tray_icon(new_state);
    if let Some(item) = TOGGLE_MENU_ITEM.get() {
        let _ = item.set_text(toggle_menu_label(new_state));
    }
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "設定を開く", true, None::<&str>)?;
    let toggle = MenuItem::with_id(
        app,
        "toggle_gesture",
        toggle_menu_label(*GESTURE_ENABLED.lock().unwrap()),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show, &toggle, &quit])?;
    let _ = TOGGLE_MENU_ITEM.set(toggle);

    let icon = load_icon_from_bytes(include_bytes!("../icons/128x128.png"))
        .or_else(|| app.default_window_icon().cloned())
        .unwrap_or_else(|| Image::new_owned(vec![0; 32 * 32 * 4], 32, 32));

    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("GestureHotkeyApp")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                // 明示終了時は必ずフックを外し、押しっぱなし状態を残さない。
                // プロセス終了でもOSがフックを自動解除するが、ここで先に
                // 明示的に外すことで通常の右クリックが即座に復元されることを保証する。
                let _ = mouse_hook::uninstall_hook();
                app.exit(0);
            }
            "show" => show_main_window(app),
            "toggle_gesture" => toggle_gesture_enabled(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                // 単純な左クリックは通常のトレイアイコンと同様に設定ウィンドウを開くだけにする。
                // 以前はここでジェスチャー全体を無音でON/OFFしていたため、
                // ユーザーがアイコンを一度クリックしただけで全トリガーが反応しなくなる
                // 不具合の原因になっていた。無効化操作は明示的なメニュー項目からのみ行う。
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    let _ = TRAY_ICON.set(Mutex::new(Some(tray)));
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_titlebar_color(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let color: COLORREF = COLORREF(0x0053493B);
    let hwnd = HWND(window.hwnd()?.0 as *mut _);

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &color as *const _ as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        )?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_titlebar_color(_window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            eprintln!("[startup] GestureHotkeyApp setup begin");
            let _ = APP_HANDLE.set(app.handle().clone());
            let tray_ready = match setup_tray(app.handle()) {
                Ok(_) => {
                    eprintln!("[startup] tray ready");
                    true
                }
                Err(err) => {
                    eprintln!("[startup] tray setup failed: {}", err);
                    false
                }
            };

            if let Some(window) = app.get_webview_window("main") {
                if let Err(err) = set_titlebar_color(&window) {
                    eprintln!("[startup] titlebar color setup failed: {}", err);
                }
            }

            if let Ok(manager) = ConfigManager::new() {
                match manager.load_config() {
                    Ok(config) => {
                        set_trajectory_enabled_internal(config.trajectory);
                        eprintln!(
                            "[startup] config loaded: triggerA={}, triggerB={}, triggerC={}, trajectory={}",
                            config.triggerA, config.triggerB, config.triggerC, config.trajectory
                        );
                    }
                    Err(e) => {
                        eprintln!("[startup] config.json validation failed: {}", e);
                        set_trajectory_enabled_internal(true);
                    }
                }
            } else {
                eprintln!("[startup] failed to initialize config manager");
            }

            if let Err(err) = trajectory_renderer::init_renderer() {
                eprintln!("[startup] trajectory renderer init failed: {}", err);
            } else {
                eprintln!("[startup] trajectory renderer ready");
            }
            if ACTION_LABEL_OVERLAY_ENABLED {
                if let Err(err) = action_label_overlay::init_overlay() {
                    eprintln!("[startup] action label overlay init failed: {}", err);
                } else {
                    eprintln!("[startup] action label overlay ready");
                }
            }

            if let Err(err) = mouse_hook::install_hook() {
                eprintln!("[startup] mouse hook install failed: {}", err);
            } else {
                eprintln!("[startup] mouse hook installed");
            }

            if !tray_ready {
                eprintln!("[startup] tray unavailable, showing main window fallback");
                show_main_window(app.handle());
            }

            eprintln!("[startup] GestureHotkeyApp setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_gestures,
            save_gesture,
            update_gesture,
            delete_gesture,
            get_license_info,
            get_config,
            save_config,
            get_actions,
            add_action,
            update_action,
            delete_action,
            set_gesture_enabled,
            is_gesture_enabled,
            get_config_file_path,
            get_gestures_file_path,
            reset_config_to_default,
            reset_gestures_to_default,
            validate_config_file,
            validate_gestures_file,
            get_config_validation_error,
            get_gestures_validation_error,
            get_version,
            get_icon_bytes,
            export_settings_bundle,
            import_settings_bundle,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_menu_label_reflects_gesture_state() {
        assert_eq!(toggle_menu_label(true), "ジェスチャーを無効化");
        assert_eq!(toggle_menu_label(false), "ジェスチャーを有効化");
    }

    #[test]
    fn trajectory_enabled_toggle_is_independent_of_action_label_overlay_flag() {
        // ACTION_LABEL_OVERLAY_ENABLED must stay false, and toggling trajectory
        // display must not depend on it in any way.
        assert!(!ACTION_LABEL_OVERLAY_ENABLED);

        set_trajectory_enabled_internal(true);
        assert!(is_trajectory_enabled_internal());
        set_trajectory_enabled_internal(false);
        assert!(!is_trajectory_enabled_internal());
        set_trajectory_enabled_internal(true);
    }

    #[test]
    fn show_action_label_for_action_is_a_no_op_when_overlay_disabled() {
        let action = Action::default();
        // Must return without touching the (unavailable in tests) native overlay window.
        show_action_label_for_action(&action);
        clear_action_label_overlay();
    }
}
