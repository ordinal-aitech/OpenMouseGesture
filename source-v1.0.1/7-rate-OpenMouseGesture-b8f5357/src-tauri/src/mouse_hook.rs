use std::sync::Mutex;
use std::time::{Duration, Instant};

use windows::{
    core::*, Win32::Foundation::*, Win32::System::Threading::*, Win32::UI::WindowsAndMessaging::*,
};

const LLMHF_INJECTED: u32 = 0x00000001;
const SMALL_MOVE_POINTS: usize = 8;
const PREVIEW_MIN_POINTS: usize = 6;
const PREVIEW_INTERVAL_MS: u64 = 16;

static HOOK_HANDLE: Mutex<Option<isize>> = Mutex::new(None);
static TRAJECTORY: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());
static IS_DRAGGING: Mutex<bool> = Mutex::new(false);
static IS_LEFT_PRESSED: Mutex<bool> = Mutex::new(false);
static GESTURE_START_WINDOW: Mutex<Option<isize>> = Mutex::new(None);
static ACTIVE_TEMPLATES: Mutex<Vec<crate::config::GestureTemplate>> = Mutex::new(Vec::new());
static ACTIVE_CONFIG: Mutex<Option<crate::config::Config>> = Mutex::new(None);
static ACTIVE_TRIGGER_SLOT: Mutex<Option<String>> = Mutex::new(None);
static LAST_PREVIEW_AT: Mutex<Option<Instant>> = Mutex::new(None);
static LAST_PREVIEW_KEY: Mutex<Option<String>> = Mutex::new(None);

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

fn active_trigger_matches_event(
    config: &crate::config::Config,
    slot: &str,
    event_type: u32,
    mouse_data: &MSLLHOOKSTRUCT,
) -> bool {
    let button = crate::trigger_button_for_slot(config, slot);
    match button {
        "right" => event_type == WM_RBUTTONUP,
        "middle" => event_type == WM_MBUTTONUP,
        "x1" => event_type == WM_XBUTTONUP && xbutton_name(mouse_data) == Some("x1"),
        "x2" => event_type == WM_XBUTTONUP && xbutton_name(mouse_data) == Some("x2"),
        _ => false,
    }
}

fn trigger_slot_for_event(
    config: &crate::config::Config,
    event_type: u32,
    mouse_data: &MSLLHOOKSTRUCT,
) -> Option<&'static str> {
    for slot in ["A", "B", "C"] {
        let button = crate::trigger_button_for_slot(config, slot);
        let matched = match button {
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

unsafe extern "system" fn mouse_hook_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let mouse_data = *(l_param.0 as *const MSLLHOOKSTRUCT);
    if (mouse_data.flags & LLMHF_INJECTED) != 0 {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let event_type = w_param.0 as u32;

    match event_type {
        WM_LBUTTONDOWN => {
            *IS_LEFT_PRESSED.lock().unwrap() = true;
        }
        WM_LBUTTONUP => {
            *IS_LEFT_PRESSED.lock().unwrap() = false;
        }
        WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
            load_active_resources();
            let config = ACTIVE_CONFIG.lock().unwrap().clone();

            if let Some(config) = config {
                if let Some(slot) = trigger_slot_for_event(&config, event_type, &mouse_data) {
                    let point = POINT {
                        x: mouse_data.pt.x,
                        y: mouse_data.pt.y,
                    };
                    let window_at_point = WindowFromPoint(point);
                    let current_window = if window_at_point != HWND::default() {
                        window_at_point
                    } else {
                        GetForegroundWindow()
                    };

                    if current_window != HWND::default() {
                        if let Some(exe_name) = get_window_exe_name(current_window) {
                            if is_ignored_by_global_config(&exe_name) {
                                return CallNextHookEx(None, n_code, w_param, l_param);
                            }
                        }
                    }

                    *GESTURE_START_WINDOW.lock().unwrap() = Some(current_window.0 as isize);
                    *IS_DRAGGING.lock().unwrap() = true;
                    *ACTIVE_TRIGGER_SLOT.lock().unwrap() = Some(slot.to_string());

                    {
                        let mut trajectory = TRAJECTORY.lock().unwrap();
                        trajectory.clear();
                        trajectory.push((mouse_data.pt.x, mouse_data.pt.y));
                    }

                    crate::set_active_trail_color(crate::color_for_trigger_slot(&config, slot));
                    clear_preview_state();
                    crate::emit_trajectory_update(&[(mouse_data.pt.x, mouse_data.pt.y)], true);
                    return LRESULT(1);
                }
            }
        }
        WM_MOUSEMOVE => {
            if *IS_DRAGGING.lock().unwrap() {
                TRAJECTORY
                    .lock()
                    .unwrap()
                    .push((mouse_data.pt.x, mouse_data.pt.y));
                crate::append_trajectory_point(mouse_data.pt.x, mouse_data.pt.y);
                update_recognition_preview(false);
            }
        }
        WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
            let was_dragging = *IS_DRAGGING.lock().unwrap();
            if was_dragging {
                let config = ACTIVE_CONFIG.lock().unwrap().clone();
                let active_slot = ACTIVE_TRIGGER_SLOT.lock().unwrap().clone();
                if let (Some(config), Some(slot)) = (config, active_slot) {
                    if active_trigger_matches_event(&config, &slot, event_type, &mouse_data) {
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

                            if let Some(gesture_name) =
                                crate::gesture_recognizer::recognize(&points, &templates)
                            {
                                if let Some(action) =
                                    crate::find_action_for_gesture(&config, &slot, &gesture_name)
                                {
                                    let target_hwnd = GESTURE_START_WINDOW
                                        .lock()
                                        .unwrap()
                                        .map(|h| HWND(h as *mut _));

                                    if let Some(hwnd) = target_hwnd {
                                        if let Some(exe_name) = get_window_exe_name(hwnd) {
                                            if let Some(ref ignore_list) = action.ignore_exe {
                                                if ignore_list
                                                    .iter()
                                                    .any(|e| e.to_lowercase() == exe_name)
                                                {
                                                    clear_preview_state();
                                                    TRAJECTORY.lock().unwrap().clear();
                                                    *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
                                                    return LRESULT(1);
                                                }
                                            }
                                        }
                                    }

                                    let _ = crate::command_executor::execute_action_with_window(
                                        action,
                                        target_hwnd,
                                        true,
                                    );
                                    crate::emit_gesture_recognized(
                                        &gesture_name,
                                        Some(&action.action_type),
                                    );
                                }
                            } else if points.len() <= SMALL_MOVE_POINTS {
                                if slot == "A"
                                    && crate::trigger_button_for_slot(&config, &slot) == "right"
                                {
                                    let mouse_pos = points[0];
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(10));
                                        crate::command_executor::send_right_click(
                                            mouse_pos.0 as i32,
                                            mouse_pos.1 as i32,
                                        );
                                    });
                                }
                            }
                        }

                        clear_preview_state();
                        TRAJECTORY.lock().unwrap().clear();
                        *ACTIVE_TRIGGER_SLOT.lock().unwrap() = None;
                        return LRESULT(1);
                    }
                }
            }
        }
        WM_MOUSEWHEEL => {
            if *IS_DRAGGING.lock().unwrap() {
                let is_left_pressed = *IS_LEFT_PRESSED.lock().unwrap();
                let wheel_delta = ((mouse_data.mouseData >> 16) & 0xFFFF) as i16;
                let wheel_direction = if wheel_delta > 0 { "up" } else { "down" };
                let wheel_trigger = if is_left_pressed {
                    format!("leftclick_wheel_{}", wheel_direction)
                } else {
                    format!("wheel_{}", wheel_direction)
                };

                if let Some(config) = ACTIVE_CONFIG.lock().unwrap().clone() {
                    if let Some(action) = config.actions.iter().find(|a| {
                        a.trigger_type == "wheel"
                            && a.wheel_trigger
                                .as_ref()
                                .map_or(false, |wt| wt == &wheel_trigger)
                    }) {
                        let target_hwnd = GESTURE_START_WINDOW
                            .lock()
                            .unwrap()
                            .map(|h| HWND(h as *mut _));
                        let _ = crate::command_executor::execute_action_with_window(
                            action,
                            target_hwnd,
                            false,
                        );
                        TRAJECTORY.lock().unwrap().clear();
                    }
                }

                return LRESULT(1);
            }
        }
        _ => {}
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

pub fn install_hook() -> Result<()> {
    unsafe {
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
        *HOOK_HANDLE.lock().unwrap() = Some(hook.0 as isize);
    }
    Ok(())
}

pub fn uninstall_hook() -> Result<()> {
    unsafe {
        let mut hook_handle = HOOK_HANDLE.lock().unwrap();
        if let Some(handle) = *hook_handle {
            UnhookWindowsHookEx(HHOOK(handle as *mut _))?;
            *hook_handle = None;
        }
    }
    Ok(())
}
