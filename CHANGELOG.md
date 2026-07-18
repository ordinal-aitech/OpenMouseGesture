# Changelog

This changelog is reconstructed from the repository's current Git history, current source code, and tracked project documentation. It records verified changes without inventing release numbers or dates that are not present in the repository.

## 2026-07-18

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
