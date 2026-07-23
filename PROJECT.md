# OpenMouseGesture Project Reference

## Purpose

OpenMouseGesture is a Windows mouse-gesture utility implemented with Tauri 2, React, TypeScript, and Rust.

This file is the canonical reference for the repository's **current specification, constraints, verified behavior, and operating state**. It is not a chronological task log. `CHANGELOG.md` is the separate canonical record of change history.

Historical work logs, superseded plans, temporary reports, and obsolete verification notes must not be kept as competing Markdown documents in this repository. Historical information belongs in the external knowledge-management system. When implementation changes, update the current facts here and record substantive history in `CHANGELOG.md`; do not create a new status Markdown file unless the user explicitly requests one.

## Current Status

- Active source: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
- Stable Windows installer handoff: `dist/windows/OpenMouseGesture-Setup-x64.exe`
- Last physically verified application baseline: commit `0458aeb` (July 21 repair cycle).
- A subsequent repair cycle implemented overlay z-order/fullscreen handling, renderer
  self-recovery, and stuck-session release recovery (see `CHANGELOG.md` 2026-07-23).
  These changes pass the automated build and test suite but still require human
  physical verification on the target machine before they are treated as verified
  behavior.
- The user will continue validating the application during normal operation and reopen work only if another material defect is found.

## Verified User Behavior

The following behavior was confirmed on the user's Windows machine:

- Gestures work with ordinary keyboard keys and other bindings that successfully register and trigger on that machine.
- Escape cancels an active or release-pending gesture without dispatching an action.
- The 120 ms confirmed-release handling substantially reduced premature single-segment dispatch during multi-direction gestures.
- A single left click on the tray icon toggles gestures enabled/disabled.
- Repeated tray left clicks alternate enabled -> disabled -> enabled.
- Tray left click does not open Settings.
- The tray right-click menu contains only `設定を開く` and `終了`.

## Known Unresolved Input Limitations

### Caps Lock

Caps Lock is not considered supported in the current product state.

On the user's machine:

- it cannot be reliably captured in Settings;
- an existing saved Caps Lock binding does not start a gesture;
- neither a physical Caps Lock press nor a mouse-vendor/remapper mapping to Caps Lock works end-to-end.

Several implementation attempts and automated tests did not reproduce the successful behavior claimed by code-level tests. Do not describe Caps Lock as working without a new physical verification on the target machine.

### Kana and Japanese IME keys

Kana and other Japanese IME-specific keys are not considered supported in the current product state.

They do not reliably register or trigger on the user's Japanese Windows environment. Do not claim support based only on frontend key names, static VK mappings, or automated tests.

### Shift, Alt, and other special keys

Special-key behavior has varied across builds. Only keys whose capture and trigger behavior are physically verified on the target machine should be used for production operation. Ordinary keys are the recommended fallback.

## Trigger Model

- Trigger slots: A, B, C.
- Gesture actions resolve by `trigger_slot + gesture_name`.
- Mouse bindings use `mouse:right`, `mouse:middle`, `mouse:x1`, and `mouse:x2`.
- Keyboard bindings use `key:<Code>` or `key:<Modifier+...+Code>`.
- Left mouse button is prohibited as a gesture-start trigger.
- Duplicate trigger assignments are resolved in A -> B -> C priority order; the first matching slot is effective.

## Gesture Stability and Cancellation

- Gesture release uses a 120 ms confirmation window to tolerate transient key-up/key-down interruptions.
- Escape is the emergency cancellation path for active and release-pending sessions.
- Cancellation must never dispatch an action.
- Disable, shutdown, settings changes, hook restart, and unrecoverable session errors must use the centralized idempotent teardown path.
- No change may reintroduce a permanently stuck red trajectory or active session.
- A per-session release watchdog polls the physical trigger state and terminates a
  session whose trigger has been physically released past a grace window, even when
  the button-up/key-up event was missed, reordered, or duplicated. The watchdog uses
  the centralized cancel path and never dispatches an action. Its grace (300 ms) is
  longer than the 120 ms confirmed-release window so the normal release path always
  terminates, and dispatches, a genuine release first.

## Trajectory Overlay Behavior

- The trajectory is drawn on a single native layered window covering the whole virtual
  desktop, created once at startup with
  `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`.
- The overlay is input-transparent and click-through, never activates or focuses, does
  not appear in the taskbar, and must not reorder, expose, or interfere with foreground
  or background applications.
- The overlay stays shown for the process lifetime; "no trajectory" is a fully
  transparent frame rather than a hidden window. This avoids `ShowWindow` show/hide
  transitions that can make the compositor briefly expose windows behind the foreground
  application while drawing.
- At gesture start the overlay re-asserts the top of the TOPMOST band using a
  z-order-only `SetWindowPos` (`SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`), so the
  trajectory appears above fullscreen/borderless windows that raised themselves after
  startup. The window's origin and size never change.
- If the overlay window or its render thread is lost, destroyed, or hidden, the next
  gesture start recreates it automatically (bounded and idempotent; no duplicate
  windows, hooks, or render loops). Trajectory rendering recovers without restarting
  the whole application.

### Fullscreen limitation

- Non-exclusive fullscreen and borderless-fullscreen windows can be overlaid, and the
  trajectory is re-asserted above them at gesture start.
- Exclusive fullscreen (Direct3D exclusive mode / hardware overlay) bypasses the desktop
  compositor and cannot be overlaid by any non-exclusive window. Trajectory visibility
  over exclusive-fullscreen applications is a Windows platform limitation, not a defect
  fixable at the overlay layer.

## Tray and Window Behavior

- Tray left click toggles gesture enabled state.
- Tray right-click menu contains only Settings and Exit.
- Disabling gestures cancels any active or pending gesture without dispatch.
- Tray icon and tooltip should reflect enabled/disabled state immediately.
- Closing the main window hides it instead of terminating the resident process.
- Tray initialization failure must not prevent hook installation; the main window is the fallback.

## Preserved Product Features

- Trigger A/B/C.
- Per-trigger trajectory colors.
- Per-slot gesture actions.
- Per-slot wheel-up and wheel-down actions.
- Grouped action list and action group reassignment.
- Keystroke, command, URL, window-operation, and literal Unicode text actions.
- Maximize/restore toggle.
- Settings export/import.
- Backup before destructive config reset or sanitation.
- Per-user Windows autostart.
- Single-instance behavior.
- Right-click short-click passthrough.
- Action-label overlay remains intentionally disabled for stability.

## Settings and Persistence

Release settings are stored under:

`%AppData%\GestureHotkeyApp\`

Primary persisted files:

- `config.json`
- `gestures.json`

Existing valid custom actions must not be replaced by bundled defaults during normal load or normalization. Destructive reset and sanitation paths must create a backup first.

## Build, Test, and Distribution

From the active source directory:

```powershell
cd C:\GitHub\open-mouse-gesture\source-v1.0.1\7-rate-OpenMouseGesture-b8f5357
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri build
```

From the repository root:

```powershell
cd C:\GitHub\open-mouse-gesture
npm run dist:windows
npm run test:dist
git diff --check
```

The distribution export copies the current release outputs to:

- `dist/windows/OpenMouseGesture-x64.exe`
- `dist/windows/OpenMouseGesture-Setup-x64.exe`
- `dist/windows/SHA256SUMS.txt`
- `dist/windows/build-info.json`

Automated tests are necessary but do not override contradictory physical results for low-level keyboard, remapper, tray, or installer behavior.

## Repository Rules

- Treat `PROJECT.md` as the canonical current-state specification and operating reference.
- Treat `CHANGELOG.md` as the canonical change history.
- Treat `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/` as the active source of truth for implementation.
- Do not create task logs, progress logs, completion reports, handoff notes, or dated status Markdown files inside the repository unless explicitly requested.
- Store historical logs and superseded decisions in the external knowledge-management system, not in competing repository documents.
- Update `PROJECT.md` when a current product fact, limitation, build flow, or operational rule changes.
- Append substantive implementation changes and their verification status to `CHANGELOG.md` while the evidence is fresh.
- Preserve unrelated workspace changes.
- Do not rewrite history or force-push for routine maintenance.
- Keep root-level distribution tooling synchronized with the actual Tauri bundle layout.
- Do not rely on old planning documents when they conflict with current code or current physical verification.

## Reopening Special-Key Investigation

If Caps Lock, Kana, or another special-key issue is reopened, diagnose it on the target machine while recording both layers at the same time:

- WebView: `event.key`, `event.code`, `location`, `repeat`.
- Windows hook: VK, scan code, extended flag, injected flag, and `dwExtraInfo`.
- Physical keyboard input versus mouse-remapper-generated input.
- Persisted trigger identity versus runtime matching identity.

Do not mark the issue solved until capture, persistence, reload, physical triggering, release, and suppression behavior are all verified on the user's machine.
