# Changelog

This file is the canonical change history for OpenMouseGesture. Current specification, limitations, and operating state are maintained in `PROJECT.md`.

## 2026-07-23 - Overlay z-order, renderer self-recovery, and stuck-session repair

### Fixed

- Trajectory overlay now re-asserts the top of the TOPMOST band at gesture start
  using a z-order-only `SetWindowPos` (`SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`),
  so the trajectory is drawn above fullscreen/borderless windows (for example
  MPC-BE in non-exclusive fullscreen) that raised themselves after the overlay was
  created. The fixed virtual-desktop origin/size invariant is preserved (no
  move/resize), so the existing no-trembling guarantee is unchanged.
- Changed the overlay to an always-shown model: "no trajectory" is now a fully
  transparent frame instead of a hidden window. Removing the per-frame and
  gesture-boundary `ShowWindow` show/hide transitions stops the DWM compositor from
  briefly exposing windows behind the foreground application while drawing over an
  overlapped browser. The overlay remains input-transparent (`WS_EX_TRANSPARENT`),
  non-activating (`WS_EX_NOACTIVATE`), click-through, and never activates, focuses,
  reorders, or exposes other windows.
- Added bounded, idempotent renderer self-recovery. If the overlay window or its
  render thread is lost, destroyed, or hidden, the next gesture start recreates it
  (guarded against duplicate windows, threads, and message loops). Trajectory
  rendering no longer becomes permanently unavailable until a full process restart.
- Added a per-session release watchdog. When the normal trigger-release path
  (button-up/key-up event) is missed, reordered, or duplicated, the watchdog polls
  the physical trigger state and terminates the stuck session through the
  centralized cancel path, which never dispatches an action. The existing 120 ms
  confirmed-release behavior is preserved; the watchdog uses a longer 300 ms grace
  so the normal release path always terminates a genuine release first.

### Verification

- Rust test suite: 149 tests passed (11 new: renderer recovery decision, overlay
  native-window configuration, and watchdog grace / physical-state probe /
  session-generation / teardown-never-dispatches coverage).
- Frontend build passed.
- Tauri build passed (release EXE and NSIS installer).
- Distribution tests: 7 passed.
- `git diff --check`: clean.
- The above is automated evidence only. The following still require human physical
  verification on the target machine and are NOT yet verified:
  - MPC-BE fullscreen trajectory visibility.
  - No background-window exposure while drawing over a foreground browser with
    overlapping windows.
  - Long-running confirmation that trajectory rendering no longer becomes
    permanently unavailable.
  - Repeated real trigger press/release confirmation that gesture sessions do not
    stick.

### Unresolved / environment-blocked

- `npm run dist:windows` could not complete in this run: the user's resident
  OpenMouseGesture application was running from `dist/windows/OpenMouseGesture-x64.exe`
  and held an exclusive lock on the previously exported EXE, so the export script
  could not clean/overwrite `dist/windows`. This is an environment lock, not a build
  defect (the Tauri release build that feeds the export succeeded). After exiting the
  tray app (right-click tray -> 終了), re-run `npm run dist:windows` to refresh the
  distribution artifacts.
- Exclusive-fullscreen (Direct3D exclusive mode / hardware overlay) applications
  bypass the desktop compositor and cannot be overlaid by any non-exclusive window;
  trajectory visibility over such windows remains a Windows platform limitation.

Source basis:

- Implementation changes are confined to
  `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`
  and `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/mouse_hook.rs`.

## 2026-07-21 - Practical repair cycle completed

### Changed

- Changed tray behavior so a single left click toggles gesture handling enabled/disabled instead of opening Settings.
- Repeated tray left clicks now alternate deterministically between enabled and disabled.
- Reduced the tray right-click menu to `設定を開く` and `終了`.
- Updated tray icon and tooltip state immediately after toggling.
- Kept disable behavior on the centralized cancel-then-uninstall path so active and release-pending gestures are cancelled without dispatch.
- Added a 120 ms confirmed-release window to reduce premature single-segment dispatch during multi-direction gestures.
- Added Escape cancellation for active and release-pending gestures.
- Added implementation support intended to improve dedicated keyboard triggers, standalone Shift/Alt identities, Settings capture mode, legacy bare-key migration, and third-party injected/remapped key handling.

### Verification

- Rust test suite: 138 tests passed.
- Frontend build passed.
- Tauri build passed.
- Windows distribution export passed.
- Distribution tests: 7 passed.
- User physically confirmed:
  - tray left click toggles enabled/disabled;
  - repeated clicks alternate correctly;
  - tray left click does not open Settings;
  - tray right-click menu contains only Settings and Exit;
  - Escape cancellation works;
  - the 120 ms release handling improved practical gesture stability;
  - ordinary keyboard keys continue to work as triggers.

### Unresolved

- Caps Lock remains unsupported in the current product state. On the target machine it could not be reliably captured, and neither physical Caps Lock nor a mouse-vendor/remapper mapping to Caps Lock triggered a gesture end-to-end.
- Kana and Japanese IME-specific keys remain unsupported on the target Japanese Windows environment.
- Code-level tests and implementation changes must not be treated as proof that Caps Lock, Kana, Shift, Alt, or other special keys work physically. A new target-machine verification is required before changing their support status.

Source basis:

- Implementation baseline commit `0458aeb`.
- User physical verification performed after the build and installer handoff.

## 2026-07-20 - Action and trigger improvements

### Changed

- Added editable action-group reassignment without deleting and recreating actions.
- Fixed wheel and gesture action dispatch while modifier keyboard triggers are held by temporarily isolating the trigger modifiers during synthetic action input.
- Changed maximize into a maximize/restore toggle.
- Added a literal Unicode `text` action type with multiline support and no clipboard dependency.
- Improved modifier-combination trigger reliability by reconciling tracked modifier state with live Windows key state.
- Made wheel actions trigger-slot-aware for A/B/C and removed the unrelated left-click-plus-wheel model.

### Verification

- Added regression coverage for action-group reassignment, modifier isolation, maximize/restore, Unicode text serialization and dispatch preparation, modifier trigger matching, and per-slot wheel actions.

## 2026-07-18 - Safety, rendering, tray, and distribution stabilization

### Fixed

- Prevented destructive reset paths from overwriting custom `config.json` or `gestures.json` without first creating a backup.
- Restored the user's prior custom action set from backup after an unguarded reset had reduced it to bundled defaults.
- Blocked left mouse button as a gesture-start trigger in UI capture, normalization, validation, import sanitation, and runtime defense.
- Replaced moving/rebasing trajectory overlay behavior with a fixed virtual-desktop overlay and synchronous gesture-start reset.
- Improved trajectory rendering with layered translucent strokes while preserving per-trigger colors.
- Fixed right-click short-click passthrough when Mouse Right is assigned to Trigger B or C.
- Hardened hook uninstall so normal right-click behavior is restored immediately.
- Added bounded hook diagnostics behind `OMG_DEBUG_HOOK=1` or `ENABLE_HOOK_DEBUG`.
- Added repository-level Windows distribution export, hashes, build metadata, and distribution tests.

### Historical note

- An intermediate July 18 tray implementation made left click open the main window and placed enable/disable in the tray menu. That behavior was superseded on July 21 by the current left-click toggle design.

## 2026-07-17 - Startup and tray reliability

- Made tray initialization non-fatal so renderer and hook setup can continue when tray creation fails.
- Added startup diagnostics for tray setup, config load, renderer initialization, and hook installation.
- Added duplicate Trigger A/B/C warnings reflecting first-match priority in A -> B -> C order.

## 2026-07-07 - Unified trigger capture

- Replaced fixed mouse-only trigger selection with unified Trigger A/B/C capture for mouse buttons and keyboard combinations.
- Added keyboard trigger parsing and low-level keyboard hook support.
- Normalized older mouse trigger values into the current `mouse:*` format.
- Preserved right-click short-click fallback for configurations using Mouse Right.

## Earlier repository history

- Repository history begins on July 3, 2026.
- Earlier commits are retained in Git history; this changelog records only behavior and milestones that can be verified from current code, documentation, and physical results.
