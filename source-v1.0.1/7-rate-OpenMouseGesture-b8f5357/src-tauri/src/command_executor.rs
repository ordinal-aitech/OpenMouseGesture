// 概要: ジェスチャー認識後のアクション実行を担当
// 入出力:
//   - 入力: Action構造体（アクションタイプ、キー、修飾子等）
//   - 出力: Result<(), String>形式の実行結果
// 例: ExecuteKeystrokeChord(["Ctrl", "C"]) -> Ctrl+Cキー送信

use crate::config::Action;

use windows::{
    core::*, Win32::Foundation::*, Win32::UI::Input::KeyboardAndMouse::*, Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*,
};

pub fn execute_action_with_window(
    action: &Action,
    target_window: Option<HWND>,
    activate_window: bool,
) -> std::result::Result<(), String> {
    let effective_window = if let Some(hwnd) = target_window {
        unsafe {
            if hwnd != HWND::default() && IsWindow(Some(hwnd)).as_bool() {
                let style = GetWindowLongW(hwnd, GWL_STYLE);
                let is_child = (style & WS_CHILD.0 as i32) != 0;

                let target_hwnd = if is_child {
                    let root = GetAncestor(hwnd, GA_ROOT);
                    eprintln!(
                        "[DEBUG] Detected child window, using root: HWND({:?}) -> HWND({:?})",
                        hwnd, root
                    );
                    root
                } else {
                    hwnd
                };

                if activate_window {
                    let _ = BringWindowToTop(target_hwnd);
                    let _ = SetForegroundWindow(target_hwnd);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }

                Some(target_hwnd)
            } else {
                None
            }
        }
    } else {
        None
    };

    match action.action_type.as_str() {
        "keystroke" => execute_keystroke(action),
        "command" => execute_command(action),
        "url" => execute_browser(action),
        "window_operation" => execute_window_operation(action, effective_window),
        _ => Err(format!("Unknown action type: {}", action.action_type)),
    }
}

fn vk_to_scan(vk: VIRTUAL_KEY) -> u16 {
    unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) as u16 }
}

fn is_extended_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_INSERT
            | VK_DELETE
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_LEFT
            | VK_RIGHT
            | VK_UP
            | VK_DOWN
            | VK_RMENU
            | VK_RCONTROL
            | VK_LWIN
            | VK_RWIN
            | VK_SNAPSHOT
            | VK_APPS
            | VK_VOLUME_UP
            | VK_VOLUME_DOWN
            | VK_VOLUME_MUTE
            | VK_MEDIA_PLAY_PAUSE
            | VK_MEDIA_STOP
            | VK_MEDIA_NEXT_TRACK
            | VK_MEDIA_PREV_TRACK
    )
}

fn is_alnum_vk(vk: VIRTUAL_KEY) -> bool {
    let code = vk.0;
    (code >= 0x41 && code <= 0x5A) || (code >= 0x30 && code <= 0x39)
}

fn is_media_key(vk: VIRTUAL_KEY) -> bool {
    matches!(
        vk,
        VK_VOLUME_UP
            | VK_VOLUME_DOWN
            | VK_VOLUME_MUTE
            | VK_MEDIA_PLAY_PAUSE
            | VK_MEDIA_STOP
            | VK_MEDIA_NEXT_TRACK
            | VK_MEDIA_PREV_TRACK
    )
}

fn map_key_name_to_vkey(name: &str) -> Option<VIRTUAL_KEY> {
    let trimmed = name.trim();
    let lower = trimmed.to_lowercase();

    match lower.as_str() {
        "ctrl" | "control" | "lctrl" | "lcontrol" => Some(VK_LCONTROL),
        "rctrl" | "rcontrol" => Some(VK_RCONTROL),
        "alt" | "lalt" => Some(VK_LMENU),
        "ralt" => Some(VK_RMENU),
        "shift" | "lshift" => Some(VK_LSHIFT),
        "rshift" => Some(VK_RSHIFT),
        "win" | "lwin" => Some(VK_LWIN),
        "rwin" => Some(VK_RWIN),
        "left" => Some(VK_LEFT),
        "right" => Some(VK_RIGHT),
        "up" => Some(VK_UP),
        "down" => Some(VK_DOWN),
        "enter" | "return" => Some(VK_RETURN),
        "esc" | "escape" => Some(VK_ESCAPE),
        "space" => Some(VK_SPACE),
        "tab" => Some(VK_TAB),
        "backspace" => Some(VK_BACK),
        "delete" => Some(VK_DELETE),
        "insert" => Some(VK_INSERT),
        "home" => Some(VK_HOME),
        "end" => Some(VK_END),
        "pageup" => Some(VK_PRIOR),
        "pagedown" => Some(VK_NEXT),
        "apps" | "menu" => Some(VK_APPS),
        "capslock" => Some(VK_CAPITAL),
        "printscreen" | "prtsc" => Some(VK_SNAPSHOT),
        "volumeup" => Some(VK_VOLUME_UP),
        "volumedown" => Some(VK_VOLUME_DOWN),
        "volumemute" => Some(VK_VOLUME_MUTE),
        "mediaplaypause" => Some(VK_MEDIA_PLAY_PAUSE),
        "mediastop" => Some(VK_MEDIA_STOP),
        "medianext" => Some(VK_MEDIA_NEXT_TRACK),
        "mediaprev" => Some(VK_MEDIA_PREV_TRACK),
        _ => {
            if trimmed.len() == 1 {
                let ch = trimmed.chars().next()?;
                if ch >= 'a' && ch <= 'z' {
                    return Some(VIRTUAL_KEY((ch as u16 - 'a' as u16) + 0x41));
                }
                if ch >= 'A' && ch <= 'Z' {
                    return Some(VIRTUAL_KEY((ch as u16 - 'A' as u16) + 0x41));
                }
                if ch >= '0' && ch <= '9' {
                    return Some(VIRTUAL_KEY((ch as u16 - '0' as u16) + 0x30));
                }
            }

            if lower.starts_with('f') {
                if let Ok(num) = lower[1..].parse::<u16>() {
                    if num >= 1 && num <= 24 {
                        return Some(VIRTUAL_KEY(VK_F1.0 + num - 1));
                    }
                }
            }

            None
        }
    }
}

fn execute_keystroke(action: &Action) -> std::result::Result<(), String> {
    let keystroke = action
        .keystroke
        .as_ref()
        .ok_or("Keystroke action requires keystroke field")?;

    let modifiers = action.modifiers.as_ref();

    let mut keys = Vec::new();
    if let Some(mods) = modifiers {
        for m in mods {
            keys.push(m.as_str());
        }
    }
    keys.push(keystroke.as_str());

    execute_keystroke_chord(&keys)
}

fn execute_keystroke_chord(keys: &[&str]) -> std::result::Result<(), String> {
    eprintln!("[DEBUG] Executing keystroke chord: {:?}", keys);

    let mut modifiers: Vec<VIRTUAL_KEY> = Vec::new();
    let mut main_key: Option<VIRTUAL_KEY> = None;

    for k in keys {
        let vk = map_key_name_to_vkey(k).ok_or_else(|| format!("Unknown key name: {}", k))?;

        if matches!(
            vk,
            VK_CONTROL
                | VK_MENU
                | VK_SHIFT
                | VK_LCONTROL
                | VK_RCONTROL
                | VK_LMENU
                | VK_RMENU
                | VK_LSHIFT
                | VK_RSHIFT
                | VK_LWIN
                | VK_RWIN
        ) {
            eprintln!("[DEBUG] Modifier key: {} -> VK=0x{:02X}", k, vk.0);
            modifiers.push(vk);
        } else {
            eprintln!("[DEBUG] Main key: {} -> VK=0x{:02X}", k, vk.0);
            main_key = Some(vk);
        }
    }

    eprintln!(
        "[DEBUG] Total modifiers: {}, main_key: {:?}",
        modifiers.len(),
        main_key.map(|v| format!("0x{:02X}", v.0))
    );

    if main_key.is_none() && modifiers.is_empty() {
        return Err("No valid keys to send".to_string());
    }

    unsafe {
        if !modifiers.is_empty() {
            let mut inputs = Vec::new();
            for &m in &modifiers {
                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: vk_to_scan(m),
                            dwFlags: KEYEVENTF_SCANCODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(m) {
                    input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(input);
            }
            let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            eprintln!(
                "[DEBUG] SendInput(mod-down): sent {}/{} inputs",
                sent,
                inputs.len()
            );
            if sent != inputs.len() as u32 {
                return Err(format!(
                    "SendInput(mod-down) failed: sent {}/{}",
                    sent,
                    inputs.len()
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(12));
        }

        if let Some(vk) = main_key {
            let mut inputs = Vec::new();

            if is_alnum_vk(vk) || is_media_key(vk) {
                let mut down_input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: vk,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(vk) {
                    down_input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(down_input);
            } else {
                let mut down_input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: vk_to_scan(vk),
                            dwFlags: KEYEVENTF_SCANCODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(vk) {
                    down_input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(down_input);
            }

            let sent_down = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            eprintln!(
                "[DEBUG] SendInput(main-down): sent {}/{} inputs, VK=0x{:02X}, use_scancode={}",
                sent_down,
                inputs.len(),
                vk.0,
                !is_alnum_vk(vk) && !is_media_key(vk)
            );
            if sent_down != inputs.len() as u32 {
                return Err(format!(
                    "SendInput(main-down) failed: sent {}/{}",
                    sent_down,
                    inputs.len()
                ));
            }

            std::thread::sleep(std::time::Duration::from_millis(10));

            inputs.clear();

            if is_alnum_vk(vk) || is_media_key(vk) {
                let mut up_input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: vk,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(vk) {
                    up_input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(up_input);
            } else {
                let mut up_input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: vk_to_scan(vk),
                            dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(vk) {
                    up_input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(up_input);
            }

            let sent_up = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            eprintln!(
                "[DEBUG] SendInput(main-up): sent {}/{} inputs",
                sent_up,
                inputs.len()
            );
            if sent_up != inputs.len() as u32 {
                return Err(format!(
                    "SendInput(main-up) failed: sent {}/{}",
                    sent_up,
                    inputs.len()
                ));
            }

            std::thread::sleep(std::time::Duration::from_millis(12));
        }

        if !modifiers.is_empty() {
            let mut inputs = Vec::new();
            for &m in modifiers.iter().rev() {
                let mut input = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: vk_to_scan(m),
                            dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                if is_extended_key(m) {
                    input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
                }
                inputs.push(input);
            }
            let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            eprintln!(
                "[DEBUG] SendInput(mod-up): sent {}/{} inputs",
                sent,
                inputs.len()
            );
            if sent != inputs.len() as u32 {
                return Err(format!(
                    "SendInput(mod-up) failed: sent {}/{}",
                    sent,
                    inputs.len()
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(8));
        }
    }

    Ok(())
}

fn execute_command(action: &Action) -> std::result::Result<(), String> {
    let command = action
        .command
        .as_ref()
        .ok_or("Execute action requires command field")?;
    eprintln!("[DEBUG] Executing command: {}", command);

    unsafe {
        let command_wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();

        let result = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(command_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        if result.0 as isize <= 32 {
            return Err(format!("ShellExecuteW failed for command: {}", command));
        }
    }

    Ok(())
}

fn execute_browser(action: &Action) -> std::result::Result<(), String> {
    let url = action
        .url
        .as_ref()
        .ok_or("Browser action requires url field")?;
    eprintln!("[DEBUG] Opening URL: {}", url);

    unsafe {
        let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();

        let result = ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(url_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );

        if result.0 as isize <= 32 {
            return Err(format!("ShellExecuteW failed for url: {}", url));
        }
    }

    Ok(())
}

pub fn send_right_click(x: i32, y: i32) {
    unsafe {
        let mut inputs = [INPUT::default(); 2];

        inputs[0].r#type = INPUT_MOUSE;
        inputs[0].Anonymous.mi = MOUSEINPUT {
            dx: x,
            dy: y,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_RIGHTDOWN,
            time: 0,
            dwExtraInfo: 0,
        };

        inputs[1].r#type = INPUT_MOUSE;
        inputs[1].Anonymous.mi = MOUSEINPUT {
            dx: x,
            dy: y,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_RIGHTUP,
            time: 0,
            dwExtraInfo: 0,
        };

        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

fn execute_window_operation(
    action: &Action,
    preferred_hwnd: Option<HWND>,
) -> std::result::Result<(), String> {
    let operation = action
        .operation
        .as_ref()
        .ok_or("Window operation requires operation field")?;

    unsafe {
        let hwnd = if let Some(h) = preferred_hwnd {
            eprintln!("[DEBUG] Using preferred window: {:?}", h);
            h
        } else {
            let fg = GetForegroundWindow();
            eprintln!("[DEBUG] Using foreground window: {:?}", fg);
            fg
        };
        if hwnd == HWND::default() || !IsWindow(Some(hwnd)).as_bool() {
            return Err("No valid target window found".to_string());
        }

        let style = GetWindowLongW(hwnd, GWL_STYLE);
        let is_child = (style & WS_CHILD.0 as i32) != 0;

        let target_hwnd = if is_child {
            let root = GetAncestor(hwnd, GA_ROOT);
            eprintln!(
                "[DEBUG] Detected child window, using root: HWND({:?}) -> HWND({:?})",
                hwnd, root
            );
            root
        } else {
            hwnd
        };

        match operation.as_str() {
            "minimize" => {
                let mut class_name = vec![0u16; 256];
                let class_len = GetClassNameW(target_hwnd, &mut class_name);
                let class_str = String::from_utf16_lossy(&class_name[..class_len as usize]);

                let mut title = vec![0u16; 256];
                let title_len = GetWindowTextW(target_hwnd, &mut title);
                let title_str = String::from_utf16_lossy(&title[..title_len as usize]);

                let style = GetWindowLongW(target_hwnd, GWL_STYLE);
                let ex_style = GetWindowLongW(target_hwnd, GWL_EXSTYLE);
                let has_appwindow = (ex_style & WS_EX_APPWINDOW.0 as i32) != 0;
                let has_toolwindow = (ex_style & WS_EX_TOOLWINDOW.0 as i32) != 0;
                let is_visible = (style & WS_VISIBLE.0 as i32) != 0;
                let is_popup = (style & WS_POPUP.0 as i32) != 0;
                let is_child = (style & WS_CHILD.0 as i32) != 0;

                let parent = GetParent(target_hwnd);
                let owner = GetWindow(target_hwnd, GW_OWNER);

                eprintln!("[DEBUG] Window info:");
                eprintln!("  HWND: {:?}", target_hwnd);
                eprintln!("  Class: {}", class_str);
                eprintln!("  Title: {}", title_str);
                eprintln!(
                    "  APPWINDOW: {}, TOOLWINDOW: {}",
                    has_appwindow, has_toolwindow
                );
                eprintln!(
                    "  VISIBLE: {}, POPUP: {}, CHILD: {}",
                    is_visible, is_popup, is_child
                );
                eprintln!("  Parent: {:?}, Owner: {:?}", parent, owner);
                eprintln!("  Style: 0x{:08X}, ExStyle: 0x{:08X}", style, ex_style);

                if !has_appwindow && !has_toolwindow {
                    let new_style = ex_style | WS_EX_APPWINDOW.0 as i32;
                    SetWindowLongW(target_hwnd, GWL_EXSTYLE, new_style);
                    let _ = SetWindowPos(
                        target_hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
                    );
                    eprintln!("[DEBUG] Added WS_EX_APPWINDOW style");
                }

                if let Err(e) = PostMessageW(
                    Some(target_hwnd),
                    WM_SYSCOMMAND,
                    WPARAM(SC_MINIMIZE as usize),
                    LPARAM(0),
                ) {
                    eprintln!("[DEBUG] PostMessage failed: {:?}, trying ShowWindow", e);
                    let result = ShowWindow(target_hwnd, SW_MINIMIZE);
                    eprintln!("[DEBUG] ShowWindow result: {}", result.as_bool());
                } else {
                    eprintln!("[DEBUG] PostMessage(WM_SYSCOMMAND, SC_MINIMIZE) sent");
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
                let is_iconic = IsIconic(target_hwnd).as_bool();
                let is_still_visible = IsWindowVisible(target_hwnd).as_bool();
                eprintln!(
                    "[DEBUG] After minimize - IsIconic: {}, IsVisible: {}",
                    is_iconic, is_still_visible
                );
            }
            "maximize" => {
                let result = ShowWindow(target_hwnd, SW_SHOWMAXIMIZED);
                if !result.as_bool() {
                    eprintln!(
                        "[DEBUG] ShowWindow returned false, but operation may have succeeded"
                    );
                }
            }
            "close" => {
                if let Err(e) = PostMessageW(Some(target_hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) {
                    eprintln!("[DEBUG] PostMessage(WM_CLOSE) failed: {:?}", e);
                    return Err(format!("Failed to close window: {:?}", e));
                } else {
                    eprintln!("[DEBUG] PostMessage(WM_CLOSE) sent");
                }
            }
            _ => return Err(format!("Unknown window operation: {}", operation)),
        }
    }

    Ok(())
}
