use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use windows::{
    core::*, Win32::Foundation::*, Win32::System::Threading::*,
    Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState, Win32::UI::WindowsAndMessaging::*,
};

const LLMHF_INJECTED: u32 = 0x00000001;
const LLKHF_INJECTED: u32 = 0x00000010;
const SMALL_MOVE_POINTS: usize = 8;
const PREVIEW_MIN_POINTS: usize = 6;
const PREVIEW_INTERVAL_MS: u64 = 16;
const DIAG_LOG_MAX_BYTES: u64 = 2_000_000;
const DIAG_MOVE_LOG_INTERVAL_MS: u64 = 200;

/// 診断ログはデフォルト無効。`OMG_DEBUG_HOOK=1` を設定して起動するか、
/// `<config_dir>/ENABLE_HOOK_DEBUG` マーカーファイルを起動前に置いた場合のみ、
/// `<config_dir>/hook_debug.log` へ追記する（サイズ上限に達したら切り詰め）。
/// マーカーファイルはコンソールを使わずにGUI起動のまま診断を有効化するためのもの。
fn diag_enabled() -> bool {
    static ENABLED: LazyLock<bool> = LazyLock::new(|| {
        if std::env::var("OMG_DEBUG_HOOK").map(|v| v == "1").unwrap_or(false) {
            return true;
        }
        crate::config::ConfigManager::new()
            .map(|manager| manager.config_dir().join("ENABLE_HOOK_DEBUG").exists())
            .unwrap_or(false)
    });
    *ENABLED
}

fn diag_log_path() -> std::path::PathBuf {
    static PATH: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
        crate::config::ConfigManager::new()
            .map(|manager| manager.config_dir().join("hook_debug.log"))
            .unwrap_or_else(|_| std::env::temp_dir().join("omg_hook_debug.log"))
    });
    PATH.clone()
}

fn diag_log(message: impl AsRef<str>) {
    if !diag_enabled() {
        return;
    }
    use std::io::Write;

    let path = diag_log_path();
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > DIAG_LOG_MAX_BYTES {
            let _ = std::fs::write(&path, b"");
        }
    }

    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(file, "[{}.{:03}] {}", now.as_secs(), now.subsec_millis(), message.as_ref());
    }
}

fn diag_should_log_move() -> bool {
    static LAST_MOVE_LOG: Mutex<Option<Instant>> = Mutex::new(None);
    let mut last = LAST_MOVE_LOG.lock().unwrap();
    let now = Instant::now();
    let should = match *last {
        Some(previous) => now.duration_since(previous) >= Duration::from_millis(DIAG_MOVE_LOG_INTERVAL_MS),
        None => true,
    };
    if should {
        *last = Some(now);
    }
    should
}

static MOUSE_HOOK_HANDLE: Mutex<Option<isize>> = Mutex::new(None);
static KEYBOARD_HOOK_HANDLE: Mutex<Option<isize>> = Mutex::new(None);
static TRAJECTORY: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());
static IS_DRAGGING: Mutex<bool> = Mutex::new(false);
static GESTURE_START_WINDOW: Mutex<Option<isize>> = Mutex::new(None);
static ACTIVE_TEMPLATES: Mutex<Vec<crate::config::GestureTemplate>> = Mutex::new(Vec::new());
static ACTIVE_CONFIG: Mutex<Option<crate::config::Config>> = Mutex::new(None);
static ACTIVE_TRIGGER_SLOT: Mutex<Option<String>> = Mutex::new(None);
static LAST_PREVIEW_AT: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_PREVIEW_KEY: Mutex<Option<String>> = Mutex::new(None);
static PRESSED_KEYS: LazyLock<Mutex<HashSet<u16>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

/// 設定側の検証をすり抜けて "left"/"mouse:left" が config に混入しても、
/// フックが左クリックをトリガーとして掴まないようにする最終防衛ライン。
/// 左クリックはここで常に None を返し、通常のクリック操作を保護する。
fn parse_mouse_trigger(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "right" | "mouse:right" => Some("right"),
        "middle" | "mouse:middle" => Some("middle"),
        "x1" | "mouse:x1" => Some("x1"),
        "x2" | "mouse:x2" => Some("x2"),
        _ => None,
    }
}

fn get_window_exe_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == 0 {
            return None;
        }

        let process_handle =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
        let mut exe_path = [0u16; 260];
        let mut size = exe_path.len() as u32;

        if QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(exe_path.as_mut_ptr()),
            &mut size,
        )
        .is_ok()
        {
            let _ = CloseHandle(process_handle);
            let path_str = String::from_utf16_lossy(&exe_path[..size as usize]);
            std::path::Path::new(&path_str)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase())
        } else {
            let _ = CloseHandle(process_handle);
            None
        }
    }
}

fn is_ignored_by_global_config(exe_name: &str) -> bool {
    if let Ok(manager) = crate::config::ConfigManager::new() {
        if let Ok(config) = manager.load_config() {
            return config
                .ignore_exe
                .iter()
                .any(|e| e.to_lowercase() == exe_name);
        }
    }
    false
}

fn clear_preview_state() {
    *LAST_PREVIEW_KEY.lock().unwrap() = None;
    *LAST_PREVIEW_AT.lock().unwrap() = None;
    crate::clear_action_label_overlay();
}

fn action_preview_key(action: &crate::config::Action) -> String {
    crate::action_key_for_action(action)
}

fn should_run_preview() -> bool {
    let mut last_preview_at = LAST_PREVIEW_AT.lock().unwrap();
    let now = Instant::now();
    let should_run = match *last_preview_at {
        Some(last) => now.duration_since(last) >= Duration::from_millis(PREVIEW_INTERVAL_MS),
        None => true,
    };

    if should_run {
        *last_preview_at = Some(now);
    }

    should_run
}

fn update_recognition_preview(force: bool) {
    if !force && !should_run_preview() {
        return;
    }

    let points_snapshot = TRAJECTORY.lock().unwrap().clone();
    let active_slot = ACTIVE_TRIGGER_SLOT.lock().unwrap().clone();

    if points_snapshot.len() < PREVIEW_MIN_POINTS || active_slot.is_none() {
        if LAST_PREVIEW_KEY.lock().unwrap().is_some() {
            clear_preview_state();
        }
        return;
    }

    let templates = ACTIVE_TEMPLATES.lock().unwrap().clone();
    let config = ACTIVE_CONFIG.lock().unwrap().clone();

    let Some(config) = config else {
        clear_preview_state();
        return;
    };
    let slot = active_slot.unwrap();

    let points: Vec<(f64, f64)> = points_snapshot
        .iter()
        .map(|(x, y)| (*x as f64, *y as f64))
        .collect();

    let Some(gesture_name) = crate::gesture_recognizer::recognize(&points, &templates) else {
        if LAST_PREVIEW_KEY.lock().unwrap().is_some() {
            clear_preview_state();
        }
        return;
    };

    let Some(action) = crate::find_action_for_gesture(&config, &slot, &gesture_name) else {
        if LAST_PREVIEW_KEY.lock().unwrap().is_some() {
            clear_preview_state();
        }
        return;
    };

    let preview_key = action_preview_key(action);
    *LAST_PREVIEW_KEY.lock().unwrap() = Some(preview_key);
    crate::show_action_label_for_action(action);
}

fn load_active_resources() {
    if let Ok(manager) = crate::config::ConfigManager::new() {
        if let Ok(config) = manager.load_config() {
            *ACTIVE_CONFIG.lock().unwrap() = Some(config);
        }
        if let Ok(templates) = manager.load_gestures() {
            *ACTIVE_TEMPLATES.lock().unwrap() = templates;
        }
    }
}

fn xbutton_name(mouse_data: &MSLLHOOKSTRUCT) -> Option<&'static str> {
    let x_button = (mouse_data.mouseData >> 16) & 0xFFFF;
    match x_button {
        1 => Some("x1"),
        2 => Some("x2"),
        _ => None,
    }
}

const MODIFIER_VK_CODES: [u16; 9] = [0x10, 0xA0, 0xA1, 0x11, 0xA2, 0xA3, 0x12, 0xA4, 0xA5];

fn modifier_pressed(keys: &HashSet<u16>, modifier: &str) -> bool {
    match modifier {
        "Shift" => [0x10, 0xA0, 0xA1].iter().any(|code| keys.contains(code)),
        "Ctrl" => [0x11, 0xA2, 0xA3].iter().any(|code| keys.contains(code)),
        "Alt" => [0x12, 0xA4, 0xA5].iter().any(|code| keys.contains(code)),
        _ => false,
    }
}

/// WH_KEYBOARD_LL は down/up イベントを取りこぼす可能性がある
/// （フォーカス切り替え、UAC昇格、フック再インストール、アプリ起動前から
/// 押されていた等）。それだけに頼ると Shift/Ctrl/Alt の内部状態が実際の
/// 物理キー状態とずれ、修飾キー付きトリガー（例: Shift+F1）だけが
/// 単一キートリガーより不安定に見える原因になる。判定の直前に
/// `GetAsyncKeyState` で実際の修飾キー状態を問い合わせ、内部トラッキング
/// 状態とマージすることで、取りこぼしがあっても修飾キー判定は常に実際の
/// 物理状態を反映する。
fn live_modifier_vks() -> HashSet<u16> {
    let mut live = HashSet::new();
    for vk in MODIFIER_VK_CODES {
        let state = unsafe { GetAsyncKeyState(vk as i32) };
        if (state as u16 & 0x8000) != 0 {
            live.insert(vk);
        }
    }
    live
}

fn keys_with_live_modifiers(keys: &HashSet<u16>) -> HashSet<u16> {
    let mut merged = keys.clone();
    merged.extend(live_modifier_vks());
    merged
}

/// 修飾キー名（"Shift"/"Ctrl"/"Alt"）に対して、`live_keys` の中で実際に押されて
/// いる方の具体的な仮想キーコード（総称コードまたは左右いずれか）を1つ返す。
/// アクション送出前に一時解除・送出後に復元するキーコードを特定するために使う。
fn modifier_vk_from_live_keys(modifier: &str, live_keys: &HashSet<u16>) -> Option<u16> {
    let candidates: &[u16] = match modifier {
        "Shift" => &[0xA0, 0xA1, 0x10],
        "Ctrl" => &[0xA2, 0xA3, 0x11],
        "Alt" => &[0xA4, 0xA5, 0x12],
        _ => &[],
    };
    candidates.iter().copied().find(|vk| live_keys.contains(vk))
}

/// 指定したキーボードトリガー文字列（例: "key:Shift+F1"）が要求する修飾キーの
/// うち、`live_keys` の時点で実際に物理的に押されているものの仮想キーコード
/// 一覧を返す。トリガーがキーボードでない、または修飾キーを持たない場合は
/// 空を返す。
fn trigger_modifier_vks_from_live_keys(trigger: &str, live_keys: &HashSet<u16>) -> Vec<u16> {
    let Some((modifiers, _code)) = crate::config::parse_keyboard_trigger(trigger) else {
        return Vec::new();
    };
    modifiers
        .iter()
        .filter_map(|modifier| modifier_vk_from_live_keys(modifier, live_keys))
        .collect()
}

/// 現在アクティブな Trigger スロットに割り当てられたキーボードトリガーが、
/// 今まさに物理的に押しっぱなしの修飾キー（Shift/Ctrl/Alt）を返す。
/// ホイールアクションやジェスチャーアクションの送出直前に呼び、送出中だけ
/// それらの修飾キーを一時的に「離した」ことにするために使う
/// （`command_executor::execute_action_isolated_from_modifiers` 参照）。
/// マウストリガーや単一キートリガー（修飾キーなし）では常に空を返す。
fn active_trigger_modifier_vks(config: &crate::config::Config, slot: &str) -> Vec<u16> {
    let trigger = crate::trigger_button_for_slot(config, slot);
    trigger_modifier_vks_from_live_keys(trigger, &live_modifier_vks())
}

fn keyboard_trigger_active(trigger: &str, keys: &HashSet<u16>) -> bool {
    let Some((modifiers, code)) = crate::config::parse_keyboard_trigger(trigger) else {
        return false;
    };
    let Some(vk_code) = crate::config::keyboard_code_to_vk(&code) else {
        return false;
    };
    keys.contains(&vk_code) && modifiers.iter().all(|modifier| modifier_pressed(keys, modifier))
}

fn keyboard_trigger_starts_on_vk(trigger: &str, vk_code: u16, keys: &HashSet<u16>) -> bool {
    let Some((modifiers, code)) = crate::config::parse_keyboard_trigger(trigger) else {
        return false;
    };
    let Some(trigger_vk) = crate::config::keyboard_code_to_vk(&code) else {
        return false;
    };
    trigger_vk == vk_code && modifiers.iter().all(|modifier| modifier_pressed(keys, modifier))
}

fn trigger_slot_for_mouse_down(
    config: &crate::config::Config,
    event_type: u32,
    mouse_data: &MSLLHOOKSTRUCT,
) -> Option<&'static str> {
    for slot in ["A", "B", "C"] {
        let trigger = crate::trigger_button_for_slot(config, slot);
        let Some(button) = parse_mouse_trigger(trigger) else {
            continue;
        };
        let matched = match button {
            "left" => event_type == WM_LBUTTONDOWN,
            "right" => event_type == WM_RBUTTONDOWN,
            "middle" => event_type == WM_MBUTTONDOWN,
            "x1" => event_type == WM_XBUTTONDOWN && xbutton_name(mouse_data) == Some("x1"),
            "x2" => event_type == WM_XBUTTONDOWN && xbutton_name(mouse_data) == Some("x2"),
            _ => false,
        };

        if matched {
            return Some(slot);
        }
    }

    None
}

fn active_mouse_trigger_matches_up(
    config: &crate::config::Config,
    slot: &str,
    event_type: u32,
    mouse_data: &MSLLHOOKSTRUCT,
) -> bool {
    let button = parse_mouse_trigger(crate::trigger_button_for_slot(config, slot));
    match button {
        Some("left") => event_type == WM_LBUTTONUP,
        Some("right") => event_type == WM_RBUTTONUP,
        Some("middle") => event_type == WM_MBUTTONUP,
        Some("x1") => event_type == WM_XBUTTONUP && xbutton_name(mouse_data) == Some("x1"),
        Some("x2") => event_type == WM_XBUTTONUP && xbutton_name(mouse_data) == Some("x2"),
        _ => false,
    }
}

fn trigger_slot_for_keyboard_down(
    config: &crate::config::Config,
    vk_code: u16,
    keys: &HashSet<u16>,
) -> Option<&'static str> {
    for slot in ["A", "B", "C"] {
        if keyboard_trigger_starts_on_vk(crate::trigger_button_for_slot(config, slot), vk_code, keys) {
            return Some(slot);
        }
    }
    None
}

fn current_cursor_point() -> POINT {
    unsafe {
        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_ok() {
            point
        } else {
            POINT { x: 0, y: 0 }
        }
    }
}

fn resolve_window_for_point(point: POINT) -> HWND {
    unsafe {
        let window_at_point = WindowFromPoint(point);
        if window_at_point != HWND::default() {
            window_at_point
        } else {
            GetForegroundWindow()
        }
    }
}

fn begin_gesture(config: &crate::config::Config, slot: &str, point: POINT, current_window: HWND) {
    diag_log(format!("gesture-session begin slot={} point=({},{})", slot, point.x, point.y));
    *GESTURE_START_WINDOW.lock().unwrap() = Some(current_window.0 as isize);
    *IS_DRAGGING.lock().unwrap() = true;
    *ACTIVE_TRIGGER_SLOT.lock().unwrap() = Some(slot.to_string());

    {
        let mut trajectory = TRAJECTORY.lock().unwrap();
        trajectory.clear();
        trajectory.push((point.x, point.y));
    }

    crate::set_active_trail_color(crate::color_for_trigger_slot(config, slot));
    clear_preview_state();
    crate::emit_trajectory_update(&[(point.x, point.y)], true);
}

/// 短いクリック（移動点数が閾値以下）で、かつそのスロットに割り当てられた
/// ボタンが右クリックの場合にのみ、合成右クリックで通常のコンテキストメニューを
/// 復元すべき座標を返す。スロットA/B/Cのどれに割り当てられていても動作する
/// （以前はスロットAに右クリックがある場合のみ復元されるバグがあった）。
fn should_replay_right_click(
    config: &crate::config::Config,
    slot: &str,
    points: &[(f64, f64)],
) -> Option<(f64, f64)> {
    if points.len() <= SMALL_MOVE_POINTS
        && parse_mouse_trigger(crate::trigger_button_for_slot(config, slot)) == Some("right")
    {
        points.first().copied()
    } else {
        None
    }
}

fn complete_gesture(config: &crate::config::Config, slot: &str) {
    diag_log(format!(
        "gesture-session end slot={} points={} gesture_enabled={}",
        slot,
        TRAJECTORY.lock().unwrap().len(),
        crate::is_gesture_enabled_internal()
    ));
    *IS_DRAGGING.lock().unwrap() = false;
    crate::emit_trajectory_update(&[], false);

    let points_snapshot = TRAJECTORY.lock().unwrap().clone();
    if !points_snapshot.is_empty() && crate::is_gesture_enabled_internal() {
        update_recognition_preview(true);
        let points: Vec<(f64, f64)> = points_snapshot
            .iter()
            .map(|(x, y)| (*x as f64, *y as f64))
            .collect();
        let templates = ACTIVE_TEMPLATES.lock().unwrap().clone();

        if let Some(gesture_name) = crate::gesture_recognizer::recognize(&points, &templates) {
            diag_log(format!("matcher result: recognized gesture={}", gesture_name));
            if let Some(action) = crate::find_action_for_gesture(config, slot, &gesture_name) {
                let target_hwnd = GESTURE_START_WINDOW
                    .lock()
                    .unwrap()
                    .map(|h| HWND(h as *mut _));

                if let Some(hwnd) = target_hwnd {
                    if let Some(exe_name) = get_window_exe_name(hwnd) {
                        if let Some(ref ignore_list) = action.ignore_exe {
                            if ignore_list.iter().any(|e| e.to_lowercase() == exe_name) {
                                clear_preview_state();
                                TRAJECTORY.lock().unwrap().clear();
                                *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
                                *GESTURE_START_WINDOW.lock().unwrap() = None;
                                return;
                            }
                        }
                    }
                }

                let modifier_vks = active_trigger_modifier_vks(config, slot);
                let dispatch_result = crate::command_executor::execute_action_isolated_from_modifiers(
                    action,
                    target_hwnd,
                    true,
                    &modifier_vks,
                );
                diag_log(format!(
                    "action-dispatch result: action={} ok={}",
                    action.name,
                    dispatch_result.is_ok()
                ));
                crate::emit_gesture_recognized(&gesture_name, Some(&action.action_type));
            } else {
                diag_log(format!("action-dispatch result: no action mapped for slot={} gesture={}", slot, gesture_name));
            }
        } else if let Some(mouse_pos) = should_replay_right_click(config, slot, &points) {
            diag_log(format!(
                "replay/pass-through: no gesture recognized, replaying right-click at ({:.0},{:.0}) for slot={}",
                mouse_pos.0, mouse_pos.1, slot
            ));
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(10));
                crate::command_executor::send_right_click(mouse_pos.0 as i32, mouse_pos.1 as i32);
            });
        } else {
            diag_log("matcher result: no gesture recognized");
        }
    }

    clear_preview_state();
    TRAJECTORY.lock().unwrap().clear();
    *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
    *GESTURE_START_WINDOW.lock().unwrap() = None;
}

unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let event_type = w_param.0 as u32;
    let is_down_up_event = matches!(
        event_type,
        WM_LBUTTONDOWN
            | WM_RBUTTONDOWN
            | WM_MBUTTONDOWN
            | WM_XBUTTONDOWN
            | WM_LBUTTONUP
            | WM_RBUTTONUP
            | WM_MBUTTONUP
            | WM_XBUTTONUP
    );
    if is_down_up_event {
        diag_log(format!("callback entered nCode={} msg={}", n_code, event_type));
    }

    let mouse_data = *(l_param.0 as *const MSLLHOOKSTRUCT);
    if (mouse_data.flags & LLMHF_INJECTED) != 0 {
        if is_down_up_event {
            diag_log("event ignored: LLMHF_INJECTED flag set (synthetic input)");
        }
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    match event_type {
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
            load_active_resources();
            let config = ACTIVE_CONFIG.lock().unwrap().clone();

            if let Some(config) = config {
                diag_log(format!(
                    "loaded trigger mapping: A={} B={} C={}",
                    config.triggerA, config.triggerB, config.triggerC
                ));
                if let Some(slot) = trigger_slot_for_mouse_down(&config, event_type, &mouse_data) {
                    diag_log(format!("trigger match: slot={}", slot));
                    let point = POINT { x: mouse_data.pt.x, y: mouse_data.pt.y };
                    let current_window = resolve_window_for_point(point);

                    if current_window != HWND::default() {
                        if let Some(exe_name) = get_window_exe_name(current_window) {
                            if is_ignored_by_global_config(&exe_name) {
                                diag_log(format!("event ignored: exe={} in ignore_exe list", exe_name));
                                return CallNextHookEx(None, n_code, w_param, l_param);
                            }
                        }
                    }

                    begin_gesture(&config, slot, point, current_window);
                    return LRESULT(1);
                } else {
                    diag_log("trigger match: none");
                }
            } else {
                diag_log("trigger match: skipped, ACTIVE_CONFIG is None");
            }
        }
        WM_MOUSEMOVE => {
            if *IS_DRAGGING.lock().unwrap() {
                TRAJECTORY.lock().unwrap().push((mouse_data.pt.x, mouse_data.pt.y));
                crate::append_trajectory_point(mouse_data.pt.x, mouse_data.pt.y);
                update_recognition_preview(false);
                if diag_should_log_move() {
                    diag_log(format!(
                        "first-movement/move accepted point=({},{}) total_points={}",
                        mouse_data.pt.x,
                        mouse_data.pt.y,
                        TRAJECTORY.lock().unwrap().len()
                    ));
                }
            }
        }
        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
            if *IS_DRAGGING.lock().unwrap() {
                let config = ACTIVE_CONFIG.lock().unwrap().clone();
                let active_slot = ACTIVE_TRIGGER_SLOT.lock().unwrap().clone();
                if let (Some(config), Some(slot)) = (config, active_slot) {
                    if active_mouse_trigger_matches_up(&config, &slot, event_type, &mouse_data) {
                        complete_gesture(&config, &slot);
                        return LRESULT(1);
                    }
                }
            }
        }
        WM_MOUSEWHEEL => {
            if *IS_DRAGGING.lock().unwrap() {
                // ホイールアクションはアクティブな Trigger スロット（A/B/C）と
                // ホイール方向のみで解決する。左クリックの押下状態には一切
                // 依存しない（旧 leftclick_wheel_* モデルは廃止済み）。
                let wheel_delta = ((mouse_data.mouseData >> 16) & 0xFFFF) as i16;
                let wheel_direction = if wheel_delta > 0 { "wheel_up" } else { "wheel_down" };
                let active_slot = ACTIVE_TRIGGER_SLOT.lock().unwrap().clone();

                if let (Some(config), Some(slot)) =
                    (ACTIVE_CONFIG.lock().unwrap().clone(), active_slot)
                {
                    diag_log(format!(
                        "wheel event: slot={} direction={}",
                        slot, wheel_direction
                    ));
                    if let Some(action) = crate::find_action_for_wheel(&config, &slot, wheel_direction) {
                        let target_hwnd = GESTURE_START_WINDOW.lock().unwrap().map(|h| HWND(h as *mut _));
                        let modifier_vks = active_trigger_modifier_vks(&config, &slot);
                        let dispatch_result = crate::command_executor::execute_action_isolated_from_modifiers(
                            action,
                            target_hwnd,
                            false,
                            &modifier_vks,
                        );
                        diag_log(format!(
                            "wheel-action-dispatch result: action={} ok={}",
                            action.name,
                            dispatch_result.is_ok()
                        ));
                        // 個々のホイールティックはジェスチャー軌跡として蓄積しない。
                        // トリガーが押されたままの連続ティックは、都度そのティック
                        // 単体のホイールアクションとして扱う。
                        TRAJECTORY.lock().unwrap().clear();
                    } else {
                        diag_log("wheel-action-dispatch result: no action mapped for slot/direction");
                    }
                }

                return LRESULT(1);
            }
        }
        _ => {}
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

unsafe extern "system" fn keyboard_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let keyboard_data = *(l_param.0 as *const KBDLLHOOKSTRUCT);
    if (keyboard_data.flags.0 & LLKHF_INJECTED) != 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let event_type = w_param.0 as u32;
    let vk_code = keyboard_data.vkCode as u16;

    match event_type {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let pressed_snapshot = {
                let mut pressed_keys = PRESSED_KEYS.lock().unwrap();
                pressed_keys.insert(vk_code);
                pressed_keys.clone()
            };
            // 内部トラッキングだけに頼らず、実際のOS修飾キー状態も合成する
            // （取りこぼしたdown/upイベントによる修飾キー判定のずれを防ぐ）。
            let effective_keys = keys_with_live_modifiers(&pressed_snapshot);

            diag_log(format!(
                "keydown vk={:#04x} sys={} pressed={:?} live_modifiers={:?}",
                vk_code,
                event_type == WM_SYSKEYDOWN,
                pressed_snapshot,
                effective_keys.difference(&pressed_snapshot).collect::<Vec<_>>()
            ));

            if !*IS_DRAGGING.lock().unwrap() {
                load_active_resources();
                let config = ACTIVE_CONFIG.lock().unwrap().clone();
                if let Some(config) = config {
                    if let Some(slot) = trigger_slot_for_keyboard_down(&config, vk_code, &effective_keys) {
                        diag_log(format!("keyboard trigger match: slot={}", slot));
                        let point = current_cursor_point();
                        let current_window = resolve_window_for_point(point);

                        if current_window != HWND::default() {
                            if let Some(exe_name) = get_window_exe_name(current_window) {
                                if is_ignored_by_global_config(&exe_name) {
                                    diag_log(format!("event ignored: exe={} in ignore_exe list", exe_name));
                                    return CallNextHookEx(None, n_code, w_param, l_param);
                                }
                            }
                        }

                        begin_gesture(&config, slot, point, current_window);
                    } else {
                        diag_log("keyboard trigger match: none");
                    }
                }
            }
        }
        WM_KEYUP | WM_SYSKEYUP => {
            let pressed_snapshot = {
                let mut pressed_keys = PRESSED_KEYS.lock().unwrap();
                pressed_keys.remove(&vk_code);
                pressed_keys.clone()
            };
            let effective_keys = keys_with_live_modifiers(&pressed_snapshot);

            diag_log(format!(
                "keyup vk={:#04x} sys={} pressed={:?}",
                vk_code,
                event_type == WM_SYSKEYUP,
                pressed_snapshot
            ));

            if *IS_DRAGGING.lock().unwrap() {
                let config = ACTIVE_CONFIG.lock().unwrap().clone();
                let active_slot = ACTIVE_TRIGGER_SLOT.lock().unwrap().clone();
                if let (Some(config), Some(slot)) = (config, active_slot) {
                    let trigger = crate::trigger_button_for_slot(&config, &slot);
                    if crate::config::parse_keyboard_trigger(trigger).is_some()
                        && !keyboard_trigger_active(trigger, &effective_keys)
                    {
                        diag_log(format!("keyboard trigger released: slot={}", slot));
                        complete_gesture(&config, &slot);
                    }
                }
            }
        }
        _ => {}
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

pub fn install_hook() -> Result<()> {
    unsafe {
        if MOUSE_HOOK_HANDLE.lock().unwrap().is_none() {
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
            *MOUSE_HOOK_HANDLE.lock().unwrap() = Some(hook.0 as isize);
        }

        if KEYBOARD_HOOK_HANDLE.lock().unwrap().is_none() {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)?;
            *KEYBOARD_HOOK_HANDLE.lock().unwrap() = Some(hook.0 as isize);
        }
    }
    Ok(())
}

pub fn uninstall_hook() -> Result<()> {
    unsafe {
        let mut mouse_handle = MOUSE_HOOK_HANDLE.lock().unwrap();
        if let Some(handle) = *mouse_handle {
            UnhookWindowsHookEx(HHOOK(handle as *mut _))?;
            *mouse_handle = None;
        }

        let mut keyboard_handle = KEYBOARD_HOOK_HANDLE.lock().unwrap();
        if let Some(handle) = *keyboard_handle {
            UnhookWindowsHookEx(HHOOK(handle as *mut _))?;
            *keyboard_handle = None;
        }
    }

    // フック解除時に、押しっぱなし・ジェスチャー進行中の状態を必ず消す。
    // これがないと、ジェスチャー中に無効化/終了した場合に次回フック導入時
    // 古い状態（掴みっぱなしのボタン等）が残ってしまう。
    PRESSED_KEYS.lock().unwrap().clear();
    *IS_DRAGGING.lock().unwrap() = false;
    *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
    *GESTURE_START_WINDOW.lock().unwrap() = None;
    TRAJECTORY.lock().unwrap().clear();
    clear_preview_state();
    crate::emit_trajectory_update(&[], false);
    diag_log("hook uninstalled: held-button/gesture state cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// mouse_hook のグローバル状態（IS_DRAGGING/TRAJECTORY 等）を共有するテストは
    /// 並列実行すると干渉するため、このロックで直列化する。
    static STATE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn config_with_triggers(a: &str, b: &str, c: &str) -> Config {
        let mut config = Config::default();
        config.triggerA = a.to_string();
        config.triggerB = b.to_string();
        config.triggerC = c.to_string();
        config
    }

    fn mouse_data_for_xbutton(x_button: u32) -> MSLLHOOKSTRUCT {
        MSLLHOOKSTRUCT {
            pt: POINT { x: 0, y: 0 },
            mouseData: x_button << 16,
            flags: 0,
            time: 0,
            dwExtraInfo: 0,
        }
    }

    #[test]
    fn parse_mouse_trigger_accepts_legacy_and_unified_right_middle_x1() {
        assert_eq!(parse_mouse_trigger("right"), Some("right"));
        assert_eq!(parse_mouse_trigger("mouse:right"), Some("right"));
        assert_eq!(parse_mouse_trigger("middle"), Some("middle"));
        assert_eq!(parse_mouse_trigger("mouse:middle"), Some("middle"));
        assert_eq!(parse_mouse_trigger("x1"), Some("x1"));
        assert_eq!(parse_mouse_trigger("mouse:x1"), Some("x1"));
        assert_eq!(parse_mouse_trigger("not-a-button"), None);
    }

    #[test]
    fn parse_mouse_trigger_never_accepts_left_click() {
        // Runtime defense-in-depth: even if config validation is bypassed
        // (hand-edited config.json, disk corruption), the hook layer itself
        // must refuse to recognize left click as a trigger button.
        assert_eq!(parse_mouse_trigger("left"), None);
        assert_eq!(parse_mouse_trigger("mouse:left"), None);
        assert_eq!(parse_mouse_trigger("MOUSE:LEFT"), None);
        assert_eq!(parse_mouse_trigger(" left "), None);
    }

    #[test]
    fn trigger_slot_for_mouse_down_never_matches_left_button_even_with_malformed_config() {
        // Simulates a config.json that bypassed validation and still contains
        // "mouse:left" for a slot. WM_LBUTTONDOWN must never resolve to a slot,
        // so normal left-click interaction always keeps working.
        let config = config_with_triggers("mouse:left", "mouse:middle", "mouse:x1");
        let empty_data = mouse_data_for_xbutton(0);
        assert_eq!(trigger_slot_for_mouse_down(&config, WM_LBUTTONDOWN, &empty_data), None);
    }

    #[test]
    fn active_mouse_trigger_matches_up_never_matches_left_button() {
        let config = config_with_triggers("mouse:left", "mouse:middle", "mouse:x1");
        let empty_data = mouse_data_for_xbutton(0);
        assert!(!active_mouse_trigger_matches_up(&config, "A", WM_LBUTTONUP, &empty_data));
    }

    #[test]
    fn trigger_slot_for_mouse_down_resolves_right_middle_x1_to_configured_slots() {
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        let empty_data = mouse_data_for_xbutton(0);

        assert_eq!(
            trigger_slot_for_mouse_down(&config, WM_RBUTTONDOWN, &empty_data),
            Some("A")
        );
        assert_eq!(
            trigger_slot_for_mouse_down(&config, WM_MBUTTONDOWN, &empty_data),
            Some("B")
        );

        let x1_data = mouse_data_for_xbutton(1);
        assert_eq!(
            trigger_slot_for_mouse_down(&config, WM_XBUTTONDOWN, &x1_data),
            Some("C")
        );

        // x2 down data must not match a slot configured for x1.
        let x2_data = mouse_data_for_xbutton(2);
        assert_eq!(trigger_slot_for_mouse_down(&config, WM_XBUTTONDOWN, &x2_data), None);
    }

    #[test]
    fn trigger_slot_for_mouse_down_returns_none_when_button_unassigned() {
        // Only A/B are bound; middle has no slot at all (mirrors a live config
        // where a slot was reassigned to a keyboard trigger).
        let config = config_with_triggers("key:Shift+F1", "mouse:right", "mouse:x1");
        let empty_data = mouse_data_for_xbutton(0);
        assert_eq!(trigger_slot_for_mouse_down(&config, WM_MBUTTONDOWN, &empty_data), None);
        assert_eq!(trigger_slot_for_mouse_down(&config, WM_RBUTTONDOWN, &empty_data), Some("B"));
    }

    #[test]
    fn active_mouse_trigger_matches_up_mirrors_down_for_right_middle_x1() {
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        let empty_data = mouse_data_for_xbutton(0);
        let x1_data = mouse_data_for_xbutton(1);

        assert!(active_mouse_trigger_matches_up(&config, "A", WM_RBUTTONUP, &empty_data));
        assert!(active_mouse_trigger_matches_up(&config, "B", WM_MBUTTONUP, &empty_data));
        assert!(active_mouse_trigger_matches_up(&config, "C", WM_XBUTTONUP, &x1_data));
        assert!(!active_mouse_trigger_matches_up(&config, "A", WM_MBUTTONUP, &empty_data));
    }

    #[test]
    fn gesture_session_down_move_up_transitions_reset_state() {
        let _guard = STATE_TEST_LOCK.lock().unwrap();
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");

        // Reset any state left by other tests.
        *IS_DRAGGING.lock().unwrap() = false;
        TRAJECTORY.lock().unwrap().clear();
        *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
        *GESTURE_START_WINDOW.lock().unwrap() = None;

        begin_gesture(&config, "A", POINT { x: 10, y: 20 }, HWND::default());
        assert!(*IS_DRAGGING.lock().unwrap());
        assert_eq!(ACTIVE_TRIGGER_SLOT.lock().unwrap().as_deref(), Some("A"));
        assert_eq!(TRAJECTORY.lock().unwrap().as_slice(), &[(10, 20)]);

        // Simulate WM_MOUSEMOVE accepting further points while dragging.
        TRAJECTORY.lock().unwrap().push((15, 25));
        TRAJECTORY.lock().unwrap().push((20, 30));
        assert_eq!(TRAJECTORY.lock().unwrap().len(), 3);

        complete_gesture(&config, "A");
        assert!(!*IS_DRAGGING.lock().unwrap());
        assert!(TRAJECTORY.lock().unwrap().is_empty());
        assert!(ACTIVE_TRIGGER_SLOT.lock().unwrap().is_none());
        assert!(GESTURE_START_WINDOW.lock().unwrap().is_none());
    }

    #[test]
    fn should_replay_right_click_fires_regardless_of_which_slot_holds_right() {
        // Regression test for the reported bug: right-click's normal context menu
        // only came back when Right happened to be bound to slot A. It must also
        // work when Right is bound to slot B or C.
        let short_click = [(100.0, 200.0)];

        let config_a = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        assert_eq!(
            should_replay_right_click(&config_a, "A", &short_click),
            Some((100.0, 200.0))
        );

        let config_b = config_with_triggers("mouse:middle", "mouse:right", "mouse:x1");
        assert_eq!(
            should_replay_right_click(&config_b, "B", &short_click),
            Some((100.0, 200.0))
        );

        let config_c = config_with_triggers("mouse:middle", "mouse:x1", "mouse:right");
        assert_eq!(
            should_replay_right_click(&config_c, "C", &short_click),
            Some((100.0, 200.0))
        );
    }

    #[test]
    fn should_replay_right_click_returns_none_once_movement_crosses_threshold() {
        // A deliberate gesture (movement above SMALL_MOVE_POINTS) must never
        // trigger a synthetic right-click, even though the button is "right".
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        let long_drag: Vec<(f64, f64)> = (0..(SMALL_MOVE_POINTS as i32 + 1))
            .map(|i| (i as f64, i as f64))
            .collect();
        assert_eq!(should_replay_right_click(&config, "A", &long_drag), None);
    }

    #[test]
    fn should_replay_right_click_returns_none_for_middle_and_x1() {
        // Middle/X1 must retain their existing (non-replayed) behavior and not
        // be regressed into emitting a synthetic right-click.
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        let short_click = [(50.0, 60.0)];
        assert_eq!(should_replay_right_click(&config, "B", &short_click), None);
        assert_eq!(should_replay_right_click(&config, "C", &short_click), None);
    }

    #[test]
    fn uninstall_hook_clears_held_button_and_gesture_state() {
        let _guard = STATE_TEST_LOCK.lock().unwrap();

        *IS_DRAGGING.lock().unwrap() = true;
        *ACTIVE_TRIGGER_SLOT.lock().unwrap() = Some("A".to_string());
        *GESTURE_START_WINDOW.lock().unwrap() = Some(1);
        TRAJECTORY.lock().unwrap().push((1, 2));
        PRESSED_KEYS.lock().unwrap().insert(0x10);

        uninstall_hook().unwrap();

        assert!(!*IS_DRAGGING.lock().unwrap());
        assert!(ACTIVE_TRIGGER_SLOT.lock().unwrap().is_none());
        assert!(GESTURE_START_WINDOW.lock().unwrap().is_none());
        assert!(TRAJECTORY.lock().unwrap().is_empty());
        assert!(PRESSED_KEYS.lock().unwrap().is_empty());
    }

    // --- Modifier keyboard trigger reliability (Shift/Ctrl/Alt + F1-F3 etc.) ---
    //
    // WH_KEYBOARD_LL reports the *specific* left/right virtual key for
    // modifiers (VK_LSHIFT/VK_RSHIFT etc.), not just the generic VK_SHIFT.
    // modifier_pressed must recognize both the generic and the left/right
    // specific codes so that neither convention silently fails to match.
    const VK_SHIFT: u16 = 0x10;
    const VK_LSHIFT: u16 = 0xA0;
    const VK_RSHIFT: u16 = 0xA1;
    const VK_CONTROL: u16 = 0x11;
    const VK_LCONTROL: u16 = 0xA2;
    const VK_MENU: u16 = 0x12; // Alt
    const VK_LMENU: u16 = 0xA4;
    const VK_F1: u16 = 0x70;
    const VK_F2: u16 = 0x71;
    const VK_F3: u16 = 0x72;
    const VK_Z: u16 = 0x5A;

    #[test]
    fn modifier_pressed_matches_generic_and_left_right_specific_vk_codes() {
        assert!(modifier_pressed(&HashSet::from([VK_SHIFT]), "Shift"));
        assert!(modifier_pressed(&HashSet::from([VK_LSHIFT]), "Shift"));
        assert!(modifier_pressed(&HashSet::from([VK_RSHIFT]), "Shift"));
        assert!(!modifier_pressed(&HashSet::from([VK_CONTROL]), "Shift"));

        assert!(modifier_pressed(&HashSet::from([VK_CONTROL]), "Ctrl"));
        assert!(modifier_pressed(&HashSet::from([VK_LCONTROL]), "Ctrl"));

        assert!(modifier_pressed(&HashSet::from([VK_MENU]), "Alt"));
        assert!(modifier_pressed(&HashSet::from([VK_LMENU]), "Alt"));
    }

    #[test]
    fn keyboard_code_to_vk_maps_f1_through_f3_correctly() {
        assert_eq!(crate::config::keyboard_code_to_vk("F1"), Some(VK_F1));
        assert_eq!(crate::config::keyboard_code_to_vk("F2"), Some(VK_F2));
        assert_eq!(crate::config::keyboard_code_to_vk("F3"), Some(VK_F3));
    }

    #[test]
    fn keyboard_trigger_starts_on_vk_requires_modifier_and_final_key_together() {
        // Shift alone (only the modifier held) must never start the gesture.
        let shift_only = HashSet::from([VK_LSHIFT]);
        assert!(!keyboard_trigger_starts_on_vk("key:Shift+F1", VK_LSHIFT, &shift_only));

        // The final key going down while Shift is held (any Shift VK
        // convention) must start it, for F1, F2, and F3 independently.
        for (trigger, vk) in [("key:Shift+F1", VK_F1), ("key:Shift+F2", VK_F2), ("key:Shift+F3", VK_F3)] {
            let keys = HashSet::from([VK_LSHIFT, vk]);
            assert!(
                keyboard_trigger_starts_on_vk(trigger, vk, &keys),
                "expected {} to start with keys {:?}",
                trigger,
                keys
            );
        }

        // Wrong final key (F2 down) must not match a Shift+F1 trigger even
        // though Shift is held.
        let wrong_key = HashSet::from([VK_LSHIFT, VK_F2]);
        assert!(!keyboard_trigger_starts_on_vk("key:Shift+F1", VK_F2, &wrong_key));
    }

    #[test]
    fn keyboard_trigger_starts_on_vk_supports_ctrl_and_alt_combinations() {
        let ctrl_f2 = HashSet::from([VK_LCONTROL, VK_F2]);
        assert!(keyboard_trigger_starts_on_vk("key:Ctrl+F2", VK_F2, &ctrl_f2));

        let alt_f3 = HashSet::from([VK_LMENU, VK_F3]);
        assert!(keyboard_trigger_starts_on_vk("key:Alt+F3", VK_F3, &alt_f3));
    }

    #[test]
    fn keyboard_trigger_starts_on_vk_single_key_trigger_ignores_modifier_state() {
        // A plain single-key trigger (e.g. "Z") must start regardless of
        // whether unrelated modifiers happen to be held, mirroring the
        // reported working behavior of single-key triggers.
        assert!(keyboard_trigger_starts_on_vk("key:KeyZ", VK_Z, &HashSet::from([VK_Z])));
        assert!(keyboard_trigger_starts_on_vk(
            "key:KeyZ",
            VK_Z,
            &HashSet::from([VK_Z, VK_LSHIFT])
        ));
    }

    #[test]
    fn keyboard_trigger_active_ends_when_modifier_or_key_released() {
        let trigger = "key:Shift+F1";

        // Both held: still active.
        assert!(keyboard_trigger_active(trigger, &HashSet::from([VK_LSHIFT, VK_F1])));

        // F1 released first: no longer active.
        assert!(!keyboard_trigger_active(trigger, &HashSet::from([VK_LSHIFT])));

        // Shift released first (F1 still down): no longer active.
        assert!(!keyboard_trigger_active(trigger, &HashSet::from([VK_F1])));

        // Neither held.
        assert!(!keyboard_trigger_active(trigger, &HashSet::new()));
    }

    #[test]
    fn trigger_slot_for_keyboard_down_resolves_independent_modifier_combinations() {
        let config = config_with_triggers("key:Shift+F1", "key:Shift+F2", "key:Shift+F3");

        assert_eq!(
            trigger_slot_for_keyboard_down(&config, VK_F1, &HashSet::from([VK_LSHIFT, VK_F1])),
            Some("A")
        );
        assert_eq!(
            trigger_slot_for_keyboard_down(&config, VK_F2, &HashSet::from([VK_LSHIFT, VK_F2])),
            Some("B")
        );
        assert_eq!(
            trigger_slot_for_keyboard_down(&config, VK_F3, &HashSet::from([VK_LSHIFT, VK_F3])),
            Some("C")
        );

        // Holding only Shift (no final key yet) must never resolve to a slot.
        assert_eq!(
            trigger_slot_for_keyboard_down(&config, VK_LSHIFT, &HashSet::from([VK_LSHIFT])),
            None
        );
    }

    #[test]
    fn trigger_slot_for_keyboard_down_single_key_trigger_still_works() {
        let config = config_with_triggers("key:KeyZ", "mouse:middle", "mouse:x1");
        assert_eq!(
            trigger_slot_for_keyboard_down(&config, VK_Z, &HashSet::from([VK_Z])),
            Some("A")
        );
    }

    #[test]
    fn live_modifier_vks_only_contains_known_modifier_codes() {
        // Cannot simulate physically holding a key in CI, but this proves the
        // live-state probe is bounded to the documented modifier VK set and
        // never panics when no modifier is physically held.
        let live = live_modifier_vks();
        for vk in &live {
            assert!(MODIFIER_VK_CODES.contains(vk));
        }
    }

    #[test]
    fn keys_with_live_modifiers_is_a_superset_of_the_tracked_snapshot() {
        let tracked = HashSet::from([VK_F1]);
        let merged = keys_with_live_modifiers(&tracked);
        assert!(merged.contains(&VK_F1));
        assert!(tracked.is_subset(&merged));
    }

    #[test]
    fn diag_log_is_a_no_op_without_the_debug_env_var() {
        // Bounded-logging guard: absent OMG_DEBUG_HOOK, diag_log must never touch disk.
        // (diag_enabled() is evaluated once via LazyLock; this test only asserts the
        // function does not panic when disabled, which is the default state in CI/prod.)
        diag_log("test message that should be dropped when disabled");
    }

    // --- Modifier-trigger wheel dispatch: physical-modifier isolation ---
    //
    // Root cause: while a modifier keyboard trigger (e.g. Shift+F1) is held, the
    // physical Shift/Ctrl/Alt key is still down at the moment a wheel action (or a
    // gesture action ended by releasing only the non-modifier key) is dispatched.
    // If dispatch sends its own keystroke unmodified, the still-held physical
    // modifier contaminates it (e.g. a plain "Down" arrow becomes "Shift+Down" in
    // the target app), which looks like "the wheel action doesn't fire" from a
    // user's perspective even though our own dispatch technically succeeded.
    // These tests cover the pure VK-resolution logic that identifies exactly which
    // modifier keys must be temporarily released/restored around dispatch.

    #[test]
    fn modifier_vk_from_live_keys_prefers_whichever_specific_or_generic_code_is_actually_held() {
        assert_eq!(
            modifier_vk_from_live_keys("Shift", &HashSet::from([VK_LSHIFT])),
            Some(VK_LSHIFT)
        );
        assert_eq!(
            modifier_vk_from_live_keys("Shift", &HashSet::from([VK_RSHIFT])),
            Some(VK_RSHIFT)
        );
        assert_eq!(
            modifier_vk_from_live_keys("Shift", &HashSet::from([VK_SHIFT])),
            Some(VK_SHIFT)
        );
        assert_eq!(modifier_vk_from_live_keys("Shift", &HashSet::new()), None);
        assert_eq!(modifier_vk_from_live_keys("NotAModifier", &HashSet::from([VK_LSHIFT])), None);
    }

    #[test]
    fn trigger_modifier_vks_from_live_keys_resolves_shift_ctrl_alt_combinations() {
        assert_eq!(
            trigger_modifier_vks_from_live_keys("key:Shift+F1", &HashSet::from([VK_LSHIFT, VK_F1])),
            vec![VK_LSHIFT]
        );
        assert_eq!(
            trigger_modifier_vks_from_live_keys("key:Ctrl+F1", &HashSet::from([VK_LCONTROL, VK_F1])),
            vec![VK_LCONTROL]
        );
        assert_eq!(
            trigger_modifier_vks_from_live_keys("key:Alt+F1", &HashSet::from([VK_LMENU, VK_F1])),
            vec![VK_LMENU]
        );
    }

    #[test]
    fn trigger_modifier_vks_from_live_keys_is_empty_for_single_key_and_mouse_triggers() {
        // Single-key triggers (e.g. "Z") and mouse triggers must never have any
        // modifier isolated/restored around them; there is nothing to contaminate.
        assert!(trigger_modifier_vks_from_live_keys("key:KeyZ", &HashSet::from([VK_Z])).is_empty());
        assert!(trigger_modifier_vks_from_live_keys("mouse:right", &HashSet::from([VK_LSHIFT])).is_empty());
    }

    #[test]
    fn trigger_modifier_vks_from_live_keys_only_returns_modifiers_actually_held_live() {
        // The trigger requires Shift, but Shift is not currently held live (e.g. it
        // was already released) -- nothing should be isolated in that case.
        assert!(trigger_modifier_vks_from_live_keys("key:Shift+F1", &HashSet::from([VK_F1])).is_empty());
    }

    #[test]
    fn active_trigger_modifier_vks_is_empty_for_mouse_trigger_slot() {
        let config = config_with_triggers("mouse:right", "mouse:middle", "mouse:x1");
        // Cannot simulate a physically-held key in CI, but a mouse-trigger slot
        // must resolve to no modifiers regardless of live keyboard state.
        assert!(active_trigger_modifier_vks(&config, "A").is_empty());
    }
}
