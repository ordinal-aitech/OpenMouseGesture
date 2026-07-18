# OpenMouseGesture Project Reference

## Purpose

OpenMouseGesture is a Windows mouse-gesture utility rebuilt as a Tauri desktop app. It is intended for users who want global gesture input, per-gesture action mapping, tray-based background operation, and portable distribution artifacts that can be rebuilt from source.

This document describes the current implementation in `main` as of July 18, 2026. It is the project-level specification for the repository, not a task log.

## Current Status

- The active application source is `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`.
- The app is a working Tauri 2 + React + TypeScript + Rust implementation for Windows.
- Recent fixes on July 17-18, 2026 hardened startup reliability, tray behavior, right-click passthrough, trajectory rendering stability, and left-click trigger safety.
- On July 18, 2026 the "reset to default" config/gestures commands were hardened to back up existing settings before overwriting, closing the gap that had let a user's custom action set be replaced by the bundled 5-action default without a recoverable copy.
- Root-level `dist/windows/` export flow exists and is intended to be the stable distribution handoff location.
- Some physical runtime checks remain desirable on a real Windows machine, especially around hardware-specific buttons and installer upgrade paths.

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

- `wheel:<wheel_trigger>`

Current runtime implementation handles these wheel paths during an active drag session:

- `wheel_up`
- `wheel_down`
- `leftclick_wheel_up`
- `leftclick_wheel_down`

The frontend also exposes `wheel_click`, `x1_button`, and `x2_button` wheel-style options, but the current low-level hook implementation in `mouse_hook.rs` only dispatches wheel actions from `WM_MOUSEWHEEL` while dragging. Documentation and future work should not assume broader runtime support without code changes.

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
- Frontend wheel-action options currently exceed what the backend hook path clearly dispatches.
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
