/*
概要: Windows起動時の自動起動に関する純粋なロジック（引数判定・起動時の
ウィンドウ表示可否判定）を扱うモジュール。実際のOSレジストリ操作は
tauri-plugin-autostart 経由で lib.rs 側から行い、ここではAppHandleを
必要としないテスト容易な判定関数のみを持つ。
入出力:
  - 入力: プロセス起動引数(`std::env::args`相当)、トレイ初期化成否
  - 出力: 自動起動経由の起動かどうか、起動時にメインウィンドウを表示すべきか
具体例:
  - `is_autostart_launch(&["app.exe".into(), "--autostart".into()])` は true
  - `should_show_main_window_on_startup(false)` はトレイ初期化失敗時のみ true
*/

/// 自動起動登録時にのみ付与するマーカー引数。
/// tauri-plugin-autostart の登録コマンドへこの引数を付けることで、
/// 通常の手動起動と自動起動を後から判別できるようにする。
pub const AUTOSTART_ARG: &str = "--autostart";

pub fn is_autostart_launch<S: AsRef<str>>(args: &[S]) -> bool {
    args.iter().any(|arg| arg.as_ref() == AUTOSTART_ARG)
}

/// トレイ初期化に失敗した場合のみ、フォールバックとしてメインウィンドウを
/// 表示する。トレイが使える限り、自動起動・手動起動を問わず常駐状態のまま
/// 前面表示や点滅を行わない。
pub fn should_show_main_window_on_startup(tray_ready: bool) -> bool {
    !tray_ready
}

/// 二重起動時、既存インスタンスへ渡された起動引数が自動起動由来であれば
/// ウィンドウを前面表示しない（自動起動と手動起動が競合しても常駐のまま
/// にする）。それ以外（ユーザーによる手動の二重起動）は既存ウィンドウを
/// 表示してよい。
pub fn should_focus_window_for_relaunch<S: AsRef<str>>(args: &[S]) -> bool {
    !is_autostart_launch(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_autostart_marker_argument() {
        let args = vec!["OpenMouseGesture.exe".to_string(), AUTOSTART_ARG.to_string()];
        assert!(is_autostart_launch(&args));
    }

    #[test]
    fn manual_launch_has_no_autostart_marker() {
        let args = vec!["OpenMouseGesture.exe".to_string()];
        assert!(!is_autostart_launch(&args));
    }

    #[test]
    fn empty_args_are_not_autostart() {
        let args: Vec<String> = Vec::new();
        assert!(!is_autostart_launch(&args));
    }

    #[test]
    fn marker_position_does_not_matter() {
        let args = vec![
            "OpenMouseGesture.exe".to_string(),
            "--some-other-flag".to_string(),
            AUTOSTART_ARG.to_string(),
        ];
        assert!(is_autostart_launch(&args));
    }

    #[test]
    fn main_window_is_shown_only_when_tray_failed() {
        assert!(should_show_main_window_on_startup(false));
        assert!(!should_show_main_window_on_startup(true));
    }

    #[test]
    fn relaunch_focus_is_suppressed_only_for_autostart_marker() {
        let autostart_args = vec!["OpenMouseGesture.exe".to_string(), AUTOSTART_ARG.to_string()];
        let manual_args = vec!["OpenMouseGesture.exe".to_string()];

        assert!(!should_focus_window_for_relaunch(&autostart_args));
        assert!(should_focus_window_for_relaunch(&manual_args));
    }
}
