# OpenMouseGesture Project Reference

## Purpose

OpenMouseGesture is a Windows mouse-gesture utility rebuilt as a Tauri desktop app. It is intended for users who want global gesture input, per-gesture action mapping, tray-based background operation, and portable distribution artifacts that can be rebuilt from source.

This document describes the current implementation in `main` as of July 20, 2026. It is the project-level specification for the repository, not a task log.

## Current Status

- The active application source is `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`.
- The app is a working Tauri 2 + React + TypeScript + Rust implementation for Windows.
- Recent fixes on July 17-18, 2026 hardened startup reliability, tray behavior, right-click passthrough, trajectory rendering stability, and left-click trigger safety.
- On July 18, 2026 the "reset to default" config/gestures commands were hardened to back up existing settings before overwriting, closing the gap that had let a user's custom action set be replaced by the bundled 5-action default without a recoverable copy.
- On July 20, 2026, two user-reported defects were fixed: modifier keyboard triggers (`Shift+F1`/`Shift+F2`/`Shift+F3` and similar) now cross-check live OS modifier state instead of relying solely on tracked down/up events, and wheel actions are now resolved by active Trigger slot (A/B/C) + wheel direction instead of by left-click state, with the legacy `leftclick_wheel_up`/`leftclick_wheel_down` model removed from runtime and UI and migrated on load.
- Later on July 20, 2026, three further user-verified gaps were fixed: wheel actions now dispatch reliably while a modifier keyboard trigger (`Shift+F1`, `Ctrl+F1`, `Alt+F1`, etc.) is held, by temporarily isolating the still-physically-held trigger modifier around dispatch; the `maximize` window operation is now a maximize/restore toggle instead of always maximizing; and a new `text` action type types saved literal Unicode text (email addresses, fixed phrases, multiline content) at the caret via `SendInput`/`KEYEVENTF_UNICODE`, kept fully distinct from `command` (external launcher).
- Root-level `dist/windows/` export flow exists and is intended to be the stable distribution handoff location.
- Some physical runtime checks remain desirable on a real Windows machine, especially around hardware-specific buttons, modifier keyboard triggers, wheel actions, and installer upgrade paths.

## Repository Layout

### Active directories

- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
  - Active app source.
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src/`
  - React frontend.
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/`
  - Rust backend, Windows hooks, config I/O, tray, renderer, packaging metadata.
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/config/`
  - Default `config.json` and `gestures.json` seeds used when no user settings exist.
- `docs/`
  - Design notes, fix records, verification logs, and earlier planning documents. Use as supporting history only when it matches current code.
- `scripts/`
  - Repository-level distribution export tooling.
- `dist/windows/`
  - Repo-level exported Windows artifacts.
- `legacy/`
  - Older WPF-based implementation kept for reference, not the active product.
- `release-v1.0.1/`
  - Release-related material, not the primary active source tree.

## Technology Stack

- Frontend: React 19, TypeScript, Vite, Zustand.
- Desktop shell: Tauri 2.
- Backend/runtime: Rust 2021 edition.
- Windows integration: `windows` crate, low-level mouse and keyboard hooks, layered Win32 overlay window, tray icon APIs, DWM titlebar color.
- Packaging: Tauri release build plus NSIS installer.
- Repo-level distribution export: Node.js scripts under `scripts/`.

## Runtime Architecture

### Process model

- Tauri hosts the settings UI and exposes commands for config, gestures, actions, validation, and settings bundle import/export.
- Rust owns the global low-level input hooks and gesture recognition flow.
- A dedicated native layered overlay window renders the gesture trajectory.
- The app normally runs as a tray application; closing the main window hides it instead of exiting.

### Startup flow

At startup the app:

1. Initializes the tray, but tray failure is non-fatal.
2. Colors the main window titlebar on Windows.
3. Loads config and enables or disables trajectory drawing based on saved settings.
4. Initializes the trajectory renderer.
5. Installs both mouse and keyboard low-level hooks.
6. Falls back to showing the main window if tray setup failed.

The July 17, 2026 tray reliability fix deliberately prevents tray initialization errors from aborting hook installation.

## Trigger Model

### Supported trigger slots

The gesture system has three configurable trigger slots:

- Trigger A
- Trigger B
- Trigger C

Each gesture action is resolved by `trigger_slot + gesture_name`, so the same gesture shape can dispatch different actions depending on which slot started it.

### Supported trigger bindings

Current accepted trigger bindings are:

- Mouse triggers: `mouse:right`, `mouse:middle`, `mouse:x1`, `mouse:x2`
- Keyboard triggers: `key:<Code>` or `key:<Modifier+...+Code>`

Legacy stored mouse values such as `right`, `middle`, `x1`, and `x2` are normalized to the current `mouse:*` format when loaded.

### Trigger A / B / C defaults

Default bindings are:

- Trigger A: `mouse:right`
- Trigger B: `mouse:middle`
- Trigger C: `mouse:x1`

Default trajectory colors are:

- Trigger A: `#FF4D4F`
- Trigger B: `#4C8DFF`
- Trigger C: `#22A06B`

### Trigger duplicate behavior

The UI warns if Trigger B duplicates Trigger A, or Trigger C duplicates an earlier slot, but it does not block the configuration.

At runtime, mouse and keyboard trigger matching is resolved in slot order `A -> B -> C`. If multiple slots use the same physical trigger, only the first matching slot is effective.

## Input Safety Rules

### Left-click prohibition

Left click is intentionally prohibited as a gesture-start trigger.

This is enforced in multiple layers:

- Settings capture UI rejects `Mouse Left`.
- Config normalization converts `left` and `mouse:left` to the unassigned state.
- Config validation treats left-click trigger bindings as invalid.
- Saving config with a left-click trigger is rejected.
- Loading a malformed on-disk config sanitizes left-click trigger values and rewrites the file.
- Importing a settings bundle sanitizes left-click trigger values before saving.
- Runtime hook parsing refuses to treat left click as a gesture trigger even if a malformed config somehow bypassed earlier defenses.

When a saved config is sanitized on load, the app creates a timestamped backup directory in the settings folder before rewriting `config.json`.

### Right-click short-click passthrough

If the configured trigger for the active slot is `Mouse Right`, the app suppresses the original low-level right-button events so it can decide whether the input was a gesture.

Current behavior:

- A deliberate gesture uses the gesture pipeline and does not replay a normal right-click.
- A short, non-gesture right-click replays a synthetic right-click at the original point so normal context menus still open.
- This passthrough now works regardless of whether `Mouse Right` is assigned to Trigger A, B, or C.

This was fixed on July 18, 2026 after a regression where ordinary right-click stopped working unless Right happened to be bound to Trigger A.

### Middle, X1, and X2 behavior

- `Mouse Middle`, `Mouse X1`, and `Mouse X2` can be assigned as gesture-start triggers.
- They do not use the synthetic right-click passthrough path.
- Releasing the same configured button ends the active gesture session.

### Keyboard trigger behavior

- Trigger A/B/C can be assigned to keyboard combinations such as `key:Shift+F1`.
- The gesture starts when the configured key goes down while the required modifiers are held.
- The gesture ends when the key combination is no longer active.
- The current pointer position becomes the gesture start point.
- Modifier state (Shift/Ctrl/Alt) is tracked from `WH_KEYBOARD_LL` down/up events, but that tracking is also cross-checked against live `GetAsyncKeyState` at the moment of each key event. This closes a reliability gap where a missed down/up event (focus changes, UAC prompts, a modifier already held before the hook installs) could desync the internally tracked modifier state from the real physical state, which showed up as `Shift+F1`/`Shift+F2`/`Shift+F3`-style combinations starting unreliably compared to single-key triggers like `Z`.
- Both `WM_KEYDOWN`/`WM_KEYUP` and `WM_SYSKEYDOWN`/`WM_SYSKEYUP` are handled, so Alt-held combinations (which Windows delivers as "system key" messages) work the same as Shift/Ctrl combinations.

### Current keyboard-trigger limitation

Keyboard-trigger input is detected with a low-level keyboard hook, but the app does not consume or suppress the original key event for other applications. The registered trigger key or key combination is still delivered to the foreground application.

This limitation should be treated as current product behavior, not a temporary doc omission.

## Gesture and Action Model

### Gesture definitions

- Gestures are stored in `gestures.json`.
- Each gesture has a unique name and a list of point coordinates.
- Default gesture templates are seeded from `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/config/default-gestures.json`.

### Action types

The backend validates these action types:

- `keystroke`
- `command`
- `url`
- `window_operation`
- `text`

`command` launches an external executable, file, URI, or shell-associated target via `ShellExecuteW`; it is not a text-entry mechanism. `text` is a distinct action type that types a saved literal Unicode string (an email address, a fixed phrase, multiline content) directly at the caret in the currently targeted application; it never launches anything and never touches the clipboard. The two are intentionally kept separate and `command` is not repurposed for text entry.

The two supported trigger types for actions are:

- `gesture`
- `wheel`

### Gesture actions

Gesture actions are keyed by:

- `gesture:<slot>:<gesture_name>`

For gesture actions:

- `trigger_slot` must be `A`, `B`, or `C`
- `gesture` must be present

### Wheel actions

Wheel actions are keyed by:

- `wheel:<trigger_slot>:<wheel_trigger>`

Wheel actions are resolved by which Trigger slot (A, B, or C) started the currently active gesture session, plus wheel direction. While Trigger A is held and the wheel moves, only the action configured for Trigger A + that direction fires; the same rule applies independently to Trigger B and Trigger C. Wheel dispatch does not depend on left-click state in any way.

Current runtime implementation handles these wheel directions during an active drag session:

- `wheel_up`
- `wheel_down`

Each wheel tick clears the accumulated trajectory (so repeated ticks don't get misread as a gesture shape) but leaves the gesture session (the held trigger) active, so repeated ticks while the trigger stays held keep dispatching per-slot wheel actions coherently. Releasing the trigger after only wheel activity (no further pointer movement) does not additionally dispatch a gesture action, since the trajectory was cleared by the last tick.

The legacy `leftclick_wheel_up`/`leftclick_wheel_down` model (an action bound to left-click held down plus wheel movement) has been removed from the runtime and the UI. On load, `Config::normalized()` migrates any existing `leftclick_wheel_up`/`leftclick_wheel_down` actions to `wheel_up`/`wheel_down` on the action's existing `trigger_slot` (defaulting to `A` if unset), reassigning to the next free slot (`A` → `B` → `C`) only if that would otherwise collide with an existing `wheel_up`/`wheel_down` action already on that slot. The action itself (name, keystroke/command/url/window operation/text) is never dropped by this migration.

The frontend no longer exposes `wheel_click`, `x1_button`, or `x2_button` as wheel-action directions; the current low-level hook implementation in `mouse_hook.rs` only ever dispatched `wheel_up`/`wheel_down` from `WM_MOUSEWHEEL` while dragging, so these were already dead options.

### Wheel actions under a modifier keyboard trigger

Wheel action lookup and dispatch (`find_action_for_wheel` + `execute_action_with_window`) never distinguished mouse triggers from keyboard triggers, so on paper a wheel action bound to Trigger A should fire the same way regardless of whether Trigger A is `mouse:right` or `key:Shift+F1`. In practice, users reported that wheel actions fired reliably with a single-key trigger (`Z`) or a mouse trigger but not with a modifier-combination keyboard trigger such as `Shift+F1` held down.

Root cause: a modifier-combination keyboard trigger keeps the physical modifier key (Shift/Ctrl/Alt, generic or left/right-specific) held down for the entire gesture session, including the moment a wheel action is dispatched mid-session. If the dispatched action itself sends a keystroke (the common case for wheel actions), that keystroke's `SendInput` calls landed on top of the still-physically-held trigger modifier, so the target application received a modifier-contaminated combination (for example a plain `Down` arrow arriving as `Shift+Down`, which many applications interpret completely differently, e.g. as a selection instead of a scroll). This made the wheel action appear not to fire at all from the user's perspective, even though internal dispatch technically succeeded.

Fix: before dispatching a wheel action (and, defensively, before dispatching a gesture action ended by releasing only the trigger's non-modifier key while a modifier is still held), `mouse_hook.rs` computes exactly which of the active trigger's required modifier virtual-key codes are still physically held via a fresh `GetAsyncKeyState` read (`trigger_modifier_vks_from_live_keys`/`active_trigger_modifier_vks`). `command_executor::execute_action_isolated_from_modifiers` then synthesizes a `KEYEVENTF_KEYUP` for exactly those keys, runs the normal action dispatch, and synthesizes the matching key-down to restore them — all synthetic events are filtered out by the existing `LLKHF_INJECTED` check in the keyboard hook, so they never desync `PRESSED_KEYS`/trigger tracking or the active gesture session. When the active trigger has no modifiers (mouse triggers, single-key triggers like `Z`), the isolated-vks list is empty and dispatch takes the original unmodified path with no added latency.

### Window operations

`window_operation` actions support `minimize`, `maximize`, and `close`, resolved against the window that started the gesture (or the foreground window if none is available), the same target/root-window resolution `execute_action_with_window` already used.

`maximize` is a maximize/restore toggle, not an unconditional maximize: `execute_window_operation` checks `IsZoomed` on the target window and calls `ShowWindow` with `SW_RESTORE` if it is already maximized, or `SW_SHOWMAXIMIZED` otherwise. The toggle decision itself is isolated into the pure `maximize_toggle_show_command(is_currently_maximized: bool)` in `command_executor.rs` so it is unit-testable without a real HWND. Saved `operation: "maximize"` config entries are unchanged and remain backward compatible; only the runtime behavior changed. The UI option label was updated to "最大化 / 元に戻す" to describe the toggle; `minimize` and `close` behavior is unchanged.

### Text input actions

`text` is a distinct action type from `command`: `command` launches an external executable/file/URI via `ShellExecuteW`, while `text` types a saved literal Unicode string directly at the caret in the currently targeted application. The two are validated, stored, and dispatched independently — `text` actions do not populate or require `command`, and vice versa.

- Config schema: `Action.text: Option<String>`, backward compatible (`#[serde(default)]`; absent on older configs, which continue to load with `text: None` for every action).
- Validation: `action_type == "text"` requires a non-blank `text` field (`Action::validate`); other action types are unaffected and do not require `text`.
- Runtime: `command_executor::execute_text_input` sends the exact configured string via `SendInput` with `KEYEVENTF_UNICODE`, one UTF-16 code unit at a time (`unicode_code_units_for_text`), which correctly reproduces ASCII, Japanese, punctuation, spaces, and surrogate-pair (astral-plane) characters without needing a virtual-key mapping. No clipboard is read or written. Line breaks (`\n`, `\r`, or `\r\n` in the stored text) are normalized to a single `U+000D` per break, matching what a physical Enter keypress sends, so multiline edit controls insert a real line break.
- Dispatch never leaves a modifier key logically stuck: `text` actions dispatched while a modifier keyboard trigger is held go through the same `execute_action_isolated_from_modifiers` path as wheel/gesture actions (see "Wheel actions under a modifier keyboard trigger" above), even though `KEYEVENTF_UNICODE` input is largely independent of Shift/Ctrl/Alt state.
- UI: `ActionEditor.tsx` exposes a `テキスト入力` action-type option with a multiline textarea; `ActionList.tsx` shows a truncated, newline-collapsed preview (`getActionDescription`) rather than the full stored text, so compact lists do not needlessly expose potentially sensitive content in full.
- Import/export: the settings bundle embeds `text` through the existing `Action`/`Config` serialization, so export/import and the custom-action-preservation rule (`ConfigManager::load_config`, `Config::normalized`) already cover it with no separate code path.

### Ignore lists

Two ignore scopes exist:

- Global `ignore_exe` in config blocks gesture start in those executables.
- Per-action `ignore_exe` skips only that specific action if the target executable matches.

## Settings, Persistence, and Live Data

### Live settings location

In release builds, the live settings directory is:

- `%AppData%\GestureHotkeyApp\`

This is derived from `dirs::config_dir()` plus `GestureHotkeyApp`.

In debug builds, settings live under the source tree:

- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/config/`

### Persisted files

Current persisted files include:

- `config.json`
- `gestures.json`

Optional runtime diagnostics may also appear there:

- `ENABLE_HOOK_DEBUG`
- `hook_debug.log`
- `backup-YYYYMMDD-HHMMSS*/`

### Legacy settings migration

On non-debug startup, if `config.json` or `gestures.json` exist beside the executable and do not yet exist in the AppData settings directory, the app copies them into the live settings directory.

This preserves users from older release layouts where settings lived next to the executable.

### Backup behavior

Backups are created under the live settings folder (`backup-YYYYMMDD-HHMMSS*/`, containing copies of `config.json` and `gestures.json` if present) whenever the app is about to perform a write that could destroy existing settings:

- Sanitizing dangerous left-click trigger values found in an existing `config.json`.
- Resetting `config.json` or `gestures.json` to the bundled defaults via the "デフォルトで上書き" (reset to default) action reachable from the validation-error dialog.

### Custom action preservation rule

`ConfigManager::load_config` only ever writes the bundled 5-action default template when `config.json` does not exist on disk (a genuinely new install/profile). A config file that exists and parses/validates successfully is normalized (missing optional fields filled, left-click triggers cleared) but its `actions` array is never replaced, truncated, or collapsed to the default set — normalization is idempotent and safe to run on every startup. If `config.json` exists but fails to parse or fails schema validation, the original file is left untouched on disk and the error is surfaced to the caller instead of being silently overwritten.

The one path that intentionally replaces the actions list is the explicit, user-initiated "reset to default" action; as of July 18, 2026 that path always backs up the current `config.json`/`gestures.json` first (see `ConfigManager::backup_before_destructive_write`), so a reset can no longer destroy a custom action set without a recoverable copy.

### Settings bundle export/import

The app can export and import a single JSON settings bundle containing:

- `config`
- `gestures`
- action groups and actions embedded within config
- trigger bindings and colors
- global `ignore_exe`

Current bundle format details:

- `formatVersion: 1`
- `appName: "GestureHotkeyApp"`
- default export filename: `GestureHotkeyApp-settings.gha.json`

Import rewrites live settings immediately and the frontend reloads config and gestures from disk.

## Trajectory Rendering

### Functional behavior

- The app draws a live trajectory only when `trajectory` is enabled.
- The active trigger slot color is selected at gesture start.
- The overlay clears when the gesture ends or hooks are uninstalled.

### Current implementation

The current renderer uses a fixed screen-space layered Win32 overlay:

- One overlay window is created once at the virtual desktop origin and size.
- The window stays fixed for the lifetime of the process.
- Stored trajectory points remain in physical screen coordinates.
- Rendering updates are posted to the renderer window; gesture-start reset is synchronous so early points cannot be wiped by a race.

This is the current solution after the July 18, 2026 jitter fixes. Older moving or rebasing overlay approaches should be considered superseded.

### Visual style

The current visual is the preserved "Design B" style:

- dense near-opaque core
- wider translucent body
- soft outer glow
- rounded joins and end caps
- color derived from the active trigger slot color

### Action label overlay

`ACTION_LABEL_OVERLAY_ENABLED = false` is a preserved product decision in current `main`.

The action-label overlay path remains in code, but it is intentionally disabled and treated as a no-op in normal runtime behavior.

## Tray and Window Behavior

- The tray icon is intended to be the primary background control surface.
- Left-clicking the tray icon shows the main window.
- The gesture enable/disable toggle is an explicit tray menu item.
- Choosing Quit uninstalls hooks before exit so normal right-click behavior is restored immediately.
- Closing the main window hides it instead of terminating the process.

## Build, Test, and Distribution Flow

### App build

From the active source directory:

```bash
cd source-v1.0.1/7-rate-OpenMouseGesture-b8f5357
npm install
npm run tauri build
```

The Tauri build produces artifacts under:

- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/bundle/nsis/`

### Repo-level distribution export

From the repository root:

```bash
npm run dist:windows
```

This does not build the app. It copies already-built release outputs into:

- `dist/windows/OpenMouseGesture-x64.exe`
- `dist/windows/OpenMouseGesture-Setup-x64.exe`
- `dist/windows/SHA256SUMS.txt`
- `dist/windows/build-info.json`

The export step verifies SHA-256 after copying and records version, build timestamp, commit SHA, and artifact hashes.

### Available tests

Verified automated checks present in the repository include:

- Rust unit tests in `config.rs`, `mouse_hook.rs`, `trajectory_renderer.rs`, and `lib.rs`
- Node tests for the repo-level distribution export helpers in `scripts/dist-windows-lib.test.mjs`

Useful commands:

```bash
cd source-v1.0.1/7-rate-OpenMouseGesture-b8f5357
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build

cd C:\GitHub\open-mouse-gesture
npm run test:dist
```

## Operational Rules

### User-impacting operational notes

- The product is Windows-specific.
- Tray failure must not be allowed to disable hook installation.
- Right-click passthrough must remain intact regardless of which slot owns `Mouse Right`.
- Left click must remain unassignable end-to-end.
- Trigger slot duplication is a warning case; runtime priority remains `A -> B -> C`.

### Git preservation rules for this repository

- Treat `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/` as the active source of truth.
- Preserve unrelated workspace changes.
- Do not rewrite history or force-push for routine documentation or maintenance work.
- Keep root-level distribution tooling in sync with the actual Tauri bundle layout.
- Do not rely on older planning documents when they conflict with current code.

## Known Limitations

- Keyboard trigger keys are still delivered to other applications.
- Real-hardware verification is still warranted for `Mouse X1` and `Mouse X2` on target machines.
- Real installer upgrade testing, including elevation and replacement of a running install, is still warranted.
- Tray failure fallback is implemented, but `os error 5` style startup environments should still be included in regression checks.

## Remaining Physical Checks

Recommended real-machine checks that remain useful:

1. Verify Trigger A/B/C behavior with actual `Right`, `Middle`, `X1`, and `X2` hardware buttons.
2. Verify keyboard trigger combinations in common applications and record the user-visible effect of unsuppressed key delivery.
3. Verify ordinary right-click context menus when `Mouse Right` is assigned to Trigger A, Trigger B, and Trigger C respectively.
4. Verify startup behavior when the tray cannot initialize and confirm hooks still install.
5. Verify installer upgrade, file-lock handling, and post-install launch behavior on a clean Windows machine.
6. Assign `Shift+F1`, `Shift+F2`, `Shift+F3` to Trigger A/B/C respectively and confirm each starts and releases its own gesture reliably (including left/right Shift), and confirm a single-key trigger such as `Z` still works.
7. Assign distinct actions to Trigger A/B/C combined with Wheel Up and Wheel Down (six combinations total) and confirm no cross-slot dispatch and no dependency on left-click state.
8. Assign Trigger A to `Shift+F1` with distinct Wheel Up/Down actions and confirm both fire reliably while the trigger is held; repeat for `Ctrl+F1` and `Alt+F1` (including left/right variants), confirm repeated wheel ticks keep dispatching, and confirm releasing the trigger afterward does not fire an unrelated gesture action.
9. On a normal (non-maximized) window such as a browser, invoke a `maximize` action twice in a row and confirm the first press maximizes and the second restores the window to its prior size and position.
10. Create `text` actions containing an email address, a Japanese sentence, punctuation/spaces, and multiline content; confirm each types the exact configured text at the caret in a target application without altering the clipboard.
