# Changelog

This file is the canonical change history for OpenMouseGesture. Current specification, limitations, and operating state are maintained in `PROJECT.md`.

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
