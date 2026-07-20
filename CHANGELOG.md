# Changelog

This changelog is reconstructed from the repository's current Git history, current source code, and tracked project documentation. It records verified changes without inventing release numbers or dates that are not present in the repository.

## 2026-07-20 (continued)

### Wheel actions under a held modifier keyboard trigger

- Root cause: wheel action lookup/dispatch itself never distinguished mouse triggers from keyboard triggers, but a modifier-combination keyboard trigger (e.g. `Shift+F1`) keeps the physical modifier key held for the entire gesture session, including the moment a wheel action dispatches mid-session. When the dispatched action sent its own keystroke, `SendInput` for that keystroke landed on top of the still-physically-held trigger modifier, so the target application received a modifier-contaminated combination (e.g. a plain `Down` arrow arriving as `Shift+Down`) instead of the intended one. This made the wheel action look like it "didn't fire," while single-key triggers (`Z`) and mouse triggers were unaffected because they never hold an extra modifier during dispatch.
- Fix: added `trigger_modifier_vks_from_live_keys`/`active_trigger_modifier_vks` in `mouse_hook.rs`, which resolve exactly which of the active trigger's required modifier virtual-key codes are still physically held via a fresh `GetAsyncKeyState` read at dispatch time. Added `command_executor::execute_action_isolated_from_modifiers`, which synthesizes a `KEYEVENTF_KEYUP` for those keys immediately before dispatch and restores them (`KEYEVENTF` key-down) immediately after, wrapping the existing `execute_action_with_window`. Wired this into both wheel-action dispatch and gesture-action dispatch in `mouse_hook.rs`. All synthetic modifier events are already filtered by the existing `LLKHF_INJECTED` check in `keyboard_hook_proc`, so they never desync `PRESSED_KEYS`/trigger tracking or the active gesture session. When the active trigger has no modifiers (mouse triggers, single-key triggers), the isolated-VK list is empty and dispatch takes the original unmodified path.
- Added regression tests covering modifier VK resolution for Shift/Ctrl/Alt (generic and left/right-specific codes), confirming single-key and mouse triggers always resolve to an empty isolation list (no behavior change for the already-working paths), and confirming a trigger's modifier is only isolated when actually held live.

Source basis:

- Current `src-tauri/src/mouse_hook.rs`, `src-tauri/src/command_executor.rs`.

### Maximize/restore toggle

- Changed the `maximize` window operation from an unconditional `ShowWindow(..., SW_SHOWMAXIMIZED)` into a maximize/restore toggle: `execute_window_operation` now checks `IsZoomed` on the resolved target window and calls `ShowWindow` with `SW_RESTORE` when already maximized, or `SW_SHOWMAXIMIZED` otherwise. The toggle decision is isolated into a pure `maximize_toggle_show_command(is_currently_maximized: bool)` so it is unit-testable without a real HWND. `minimize` and `close` are unchanged. Saved `operation: "maximize"` config entries continue to work with no config migration needed; only the runtime behavior changed. Updated the action-editor label to "最大化 / 元に戻す" to describe the toggle.
- Added regression tests covering both toggle directions.

Source basis:

- Current `src-tauri/src/command_executor.rs`, `src/components/actions/ActionEditor.tsx`, `src/components/actions/ActionList.tsx`.

### New `text` action type (Unicode literal text input)

- Added a `text` action type, distinct from `command`: `command` remains an external executable/file/URI launcher via `ShellExecuteW` and was not changed or repurposed. `text` types a saved literal Unicode string directly at the caret of the currently targeted application.
- Config schema: `Action.text: Option<String>`, `#[serde(default)]` for backward compatibility — configs saved before this change (with no `text` field at all) continue to load, with every action's `text` defaulting to `None`. `Action::validate` requires a non-blank `text` field when `action_type == "text"`; other action types are unaffected.
- Runtime: added `command_executor::execute_text_input`, which sends the configured string via `SendInput`/`KEYEVENTF_UNICODE`, one UTF-16 code unit at a time (`unicode_code_units_for_text`), correctly reproducing ASCII, Japanese, punctuation, spaces, and astral-plane characters (surrogate pairs) without a virtual-key mapping and without reading or writing the clipboard. Line breaks (`\n`/`\r`/`\r\n`) are normalized to a single `U+000D` per break, matching what a physical Enter keypress sends, so multiline edit controls insert a real line break. `text` actions dispatched while a modifier keyboard trigger is held go through the same `execute_action_isolated_from_modifiers` path as wheel/gesture actions, for consistency with the fix above.
- Frontend: `ActionEditor.tsx` adds a `テキスト入力` action-type option with a multiline textarea and its own required-field validation; `ActionList.tsx` shows a truncated, newline-collapsed preview instead of the full stored text in compact lists. `types/index.ts` adds `Action.text` and the `"text"` action type. Import/export and the existing custom-action-preservation rule cover the new field automatically through the existing `Action`/`Config` serialization, with no separate code path.
- Added regression tests covering: `text` action_type validation acceptance, rejection of missing/blank text, Japanese/punctuation/multiline content, non-interference with `command` (each type validates independently and neither requires the other's field), serialization/reload round-tripping of Unicode and line breaks, backward-compatible loading of a config file with no `text` field at all, and pure Unicode-encoding coverage (ASCII, Japanese, astral-plane surrogate pairs, all three line-break styles, empty-text edge case) isolated from the real `SendInput` call so tests never inject live keystrokes.

Source basis:

- Current `src-tauri/src/config.rs`, `src-tauri/src/command_executor.rs`, `src/types/index.ts`, `src/components/actions/ActionEditor.tsx`, `src/components/actions/ActionList.tsx`.

## 2026-07-20

### Modifier keyboard trigger reliability fix

- Root cause: `keyboard_hook_proc` tracked Shift/Ctrl/Alt modifier state purely from `WH_KEYBOARD_LL` down/up events accumulated in a process-global `PRESSED_KEYS` set. That tracking has no way to recover if a down/up event is ever missed (a modifier already held before the hook installs, a focus change, a UAC prompt, or any other event the low-level hook doesn't see), which silently desyncs the internally tracked modifier state from the real physical key state. Because single-key triggers (e.g. `Z`) never consult modifier state at all, they were unaffected, while modifier combinations such as `Shift+F1`/`Shift+F2`/`Shift+F3` appeared to work "sometimes" depending on whether the internal tracking happened to still be in sync — matching the reported symptom.
- Fix: added `live_modifier_vks()`/`keys_with_live_modifiers()` in `mouse_hook.rs`, which cross-check the tracked key set against live `GetAsyncKeyState` for all documented Shift/Ctrl/Alt virtual-key codes (both the generic and the left/right-specific codes) at the moment of each keyboard hook event, and used the merged result for both trigger-start matching (`trigger_slot_for_keyboard_down`) and trigger-release matching (`keyboard_trigger_active`). This makes modifier detection self-healing against any missed event instead of depending entirely on having observed every prior down/up.
- Added bounded `diag_log` tracing to the keyboard hook path (previously only the mouse hook path was traced), covering key down/up VK codes, live-modifier deltas, trigger match/no-match, and trigger-release, gated by the same `OMG_DEBUG_HOOK`/`ENABLE_HOOK_DEBUG` mechanism used elsewhere (default off, bounded log size).
- Added regression tests covering `Shift+F1`/`Shift+F2`/`Shift+F3` start/hold/release behavior, generic vs. left/right-specific modifier VK codes, Ctrl/Alt combinations, and confirmation that single-key triggers remain unaffected by modifier-matching logic.

Source basis:

- Current `src-tauri/src/mouse_hook.rs`.

### Trigger-slot-aware wheel actions; left-click+wheel model removed

- Root cause: wheel action lookup ignored which Trigger slot (A/B/C) started the active gesture session and matched only on `wheel_trigger` globally, and additionally branched into a `leftclick_wheel_up`/`leftclick_wheel_down` model keyed off whether the left mouse button happened to be held during the wheel event — a model unrelated to the configured trigger slots and inconsistent with the rest of the trigger system.
- Fix: wheel actions are now keyed by `wheel:<trigger_slot>:<wheel_direction>` and resolved via a new `find_action_for_wheel(config, trigger_slot, wheel_direction)` (mirroring `find_action_for_gesture`), using the slot that started the active gesture session. Wheel dispatch in `mouse_hook.rs` no longer reads or depends on left-click state at all; the `IS_LEFT_PRESSED` tracking state was removed as dead weight once nothing consulted it.
- Config schema: `Action.trigger_slot` (already used by gesture actions) is now also used by wheel actions, defaulting to `A` when unset. `Action::validate` now requires a valid `trigger_slot` for wheel actions, matching the existing gesture-action rule.
- Legacy migration: `Config::normalized()` now migrates any existing `leftclick_wheel_up`/`leftclick_wheel_down` actions to `wheel_up`/`wheel_down` on their existing (or defaulted `A`) `trigger_slot`, reassigning to the next free slot (`A` → `B` → `C`) only if that would otherwise collide with an existing `wheel_up`/`wheel_down` action already on that slot. No action is ever dropped by this migration, even in the (contrived) case where all three slots are already occupied.
- Frontend: `ActionEditor.tsx` now shows the Trigger Slot selector for wheel actions as well as gesture actions, and the wheel-trigger dropdown was narrowed to `wheel_up`/`wheel_down` only (removing the never-functional `wheel_click`/`x1_button`/`x2_button`/`leftclick_wheel_up`/`leftclick_wheel_down` options). `getActionKey`/`getActionDisplayTrigger` in `utils/actionKey.ts` and the trigger pill in `ActionList.tsx` were updated to reflect the slot-aware wheel key.
- Added regression tests covering per-slot/per-direction wheel action lookup (all six A/B/C × up/down combinations), no cross-slot dispatch, first-match-wins behavior on duplicate slot+direction entries, and legacy migration (slot defaulting, collision avoidance, and action retention when every slot is occupied).

Source basis:

- Current `src-tauri/src/mouse_hook.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/config.rs`, `src/components/actions/ActionEditor.tsx`, `src/components/actions/ActionList.tsx`, `src/utils/actionKey.ts`, `src/types/index.ts`.

## 2026-07-18

### Config reset-to-default data-loss fix

- Root cause: the `reset_config_to_default` and `reset_gestures_to_default` Tauri commands (reachable from the "デフォルトで上書き" button in `ValidationErrorDialog`, shown when `validate_config_file`/`validate_gestures_file` fails) overwrote the live `config.json`/`gestures.json` with the bundled 5-action default template with no backup step. A user's richer custom action set (a 21-action config built up over an editing session) was lost this way, because the reset path had no recoverable copy.
- Fix: added `ConfigManager::backup_before_destructive_write`, which reuses the same timestamped `backup-YYYYMMDD-HHMMSS/` convention already used by left-click sanitization, and wired both reset commands to call it before writing the default template. The reset action itself is unchanged (it remains an explicit, user-confirmed action), but it can no longer destroy data irrecoverably.
- Confirmed the rest of the config load/save path (`ConfigManager::load_config`, `Config::normalized`, `migrate_legacy_release_files`) was already safe: defaults are only ever written when `config.json` does not exist on disk, normalization never touches the `actions` array's contents, and parse/validation failures are surfaced as errors without touching the file on disk.
- Restored the user's prior 21-action custom gesture/action set (from `%AppData%\GestureHotkeyApp\backup-20260718-115125\config.json`) into the live config, preserving current safe trigger-button assignments and left-click-blocking normalization; the live config had been silently reduced to the 5-action default via the unguarded reset path described above.
- Added regression tests in `src-tauri/src/config.rs` covering: a 20/21-action custom config surviving load/save unchanged, missing optional fields being filled without dropping actions, malformed config being preserved (not silently replaced) with the error surfaced, first-run default creation, idempotent repeated startup loads, defaults never overriding a valid richer custom set, left-click sanitation preserving the actions array, backup-before-destructive-write capturing the full custom set, and serialization/reload round-tripping restored actions.

Source basis:

- `src-tauri/src/config.rs`, `src-tauri/src/lib.rs`, `src/components/common/ValidationErrorDialog.tsx`.

### Left-click trigger safety hardening

- Blocked left-click gesture triggers end-to-end.
- Added UI rejection for `Mouse Left` during trigger capture.
- Sanitized `left` and `mouse:left` to the unassigned state during config normalization.
- Rejected attempts to save a config containing left-click trigger bindings.
- Sanitized malformed on-disk configs and imported settings bundles containing left-click triggers.
- Added backup creation for rewritten live settings when sanitizing stored left-click trigger values.
- Added runtime defense so the hook layer itself never treats left click as a valid trigger, even if earlier validation is bypassed.

Source basis:

- Commit `162d1c2` on July 18, 2026.
- Current `src-tauri/src/config.rs`, `src-tauri/src/mouse_hook.rs`, and `src/components/settings/SettingsTab.tsx`.

### Fixed-screen-space trajectory stability

- Replaced moving/rebasing overlay logic with a single overlay window fixed to the virtual desktop origin and size.
- Removed dependence on mid-gesture window movement or resize to follow the path.
- Made gesture-start trajectory reset synchronous so early points cannot be wiped by a render-order race.

Source basis:

- Commit `30de50a` on July 18, 2026.
- Current `src-tauri/src/lib.rs` and `src-tauri/src/trajectory_renderer.rs`.

### Intermediate trajectory jitter stabilization

- Earlier the same day, stabilized diagonal jitter by anchoring the overlay window per gesture and only growing its bounds.
- This was later superseded by the fixed virtual-desktop overlay approach, but it remains an important milestone in the renderer stabilization work.

Source basis:

- Commit `4fac0fe` on July 18, 2026.

### Trajectory visual improvements

- Reworked the trajectory from a flat opaque line into a layered translucent stroke using glow, body, and dense core layers.
- Preserved per-trigger color rendering instead of a single hardcoded line color.
- Tuned the final "Design B" appearance toward a darker dense core and softer outward glow.
- Consolidated renderer state to eliminate point/bounds snapshot races.

Source basis:

- Commits `ce3d58a` and `8b47b41` on July 18, 2026.
- Current renderer constants and tests in `src-tauri/src/trajectory_renderer.rs`.

### Right-click passthrough fixes

- Fixed ordinary right-click context-menu passthrough when `Mouse Right` is assigned to Trigger B or Trigger C, not just Trigger A.
- Centralized the short-click-versus-gesture replay decision so right-click fallback applies to whichever slot owns the right button.
- Hardened hook uninstall to clear active gesture state and restore normal right-click behavior immediately on quit.

Source basis:

- Commit `c747dc4` on July 18, 2026.
- Current `src-tauri/src/mouse_hook.rs` and `src-tauri/src/lib.rs`.

### Tray left-click behavior fix and diagnostics

- Changed tray icon left-click from a silent global gesture disable action into normal "show main window" behavior.
- Moved gesture enable/disable to an explicit tray menu item.
- Added bounded hook diagnostics gated by `OMG_DEBUG_HOOK=1` or `ENABLE_HOOK_DEBUG` in the config directory.
- Added throttled `hook_debug.log` output for hook-path investigation.

Source basis:

- Commit `00fc989` on July 18, 2026.
- Current `src-tauri/src/lib.rs` and `src-tauri/src/mouse_hook.rs`.

### Repository/workspace distribution layout

- Added a root `package.json` for repository-level scripts.
- Added `npm run dist:windows` to export the already-built Tauri release binary and NSIS installer into a stable root `dist/windows/` layout.
- Added exported artifact hashing and `build-info.json` metadata generation.
- Documented the root-level distribution flow in the root README and `dist/README.md`.

Source basis:

- Commit `ce3d58a` on July 18, 2026.
- Current root `package.json`, `scripts/dist-windows.mjs`, and `dist/README.md`.

### Build and test milestones

- Added or expanded unit coverage around trajectory rendering, trigger parsing, gesture-session state transitions, and left-click sanitization.
- Repo-level distribution export helper tests were added for the packaging path.

Source basis:

- Commits `ce3d58a`, `00fc989`, `8b47b41`, and `162d1c2`.

## 2026-07-17

### Startup and tray reliability

- Made tray initialization non-fatal so the app can continue to initialize the trajectory renderer and install hooks even if the tray fails.
- Added startup logging that makes tray setup, config load, renderer init, and hook installation observable.
- Added UI warnings when Trigger B or Trigger C duplicate an earlier trigger slot because runtime dispatch only uses the first matching slot in `A -> B -> C` order.

Source basis:

- Commit `b0192a9` on July 17, 2026.
- Current `src-tauri/src/lib.rs`, `src-tauri/src/config.rs`, and `src/components/settings/SettingsTab.tsx`.

## 2026-07-07

### Unified mouse and keyboard trigger capture

- Replaced fixed mouse-only trigger selection with a unified Trigger A/B/C capture model that stores actual mouse buttons or keyboard combinations.
- Added keyboard trigger parsing and low-level keyboard hook support.
- Normalized older stored mouse trigger values into the current `mouse:*` format.
- Updated settings UI, default config, and runtime hook behavior to support mixed mouse and keyboard trigger bindings.
- Preserved the existing right-click short-click fallback for configurations that still use `Mouse Right`.

Source basis:

- Commit `b695482` on July 7, 2026.
- Current `src-tauri/src/config.rs`, `src-tauri/src/mouse_hook.rs`, and `src/components/settings/SettingsTab.tsx`.

## Earlier repository history

### Initial repository import and early tracked state

- The repository history begins on July 3, 2026 with the initial tracked state and a small number of early follow-up commits.
- Those early commits do not provide a reliable user-facing feature narrative on their own, so this changelog intentionally avoids overstating their functional meaning beyond what is clearly verifiable in current code and docs.

Source basis:

- Commits beginning with `0950de3` on July 3, 2026.

## Notes on scope

- This changelog prioritizes verified behavior and structural milestones over exhaustive file-by-file history.
- Where an intermediate implementation was later replaced on the same day, both steps are recorded only when the history clearly shows they mattered to the final result.
