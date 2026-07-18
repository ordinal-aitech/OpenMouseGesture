# Open Mouse Gesture — repo-level dist/windows export + trajectory glow refinement

Date: 2026-07-18
Working directory: `C:\GitHub\open-mouse-gesture`
Primary source: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357`
Reference docs read: `ai-executor/AGENTS.md`, `00_ROUTING.md`, `01_AI_ORCHESTRATOR_RUNTIME.md`, `02_SOFTWARE_IMPLEMENTATION_EXECUTION.md`, repo `README.md`, `CLAUDE.md`, `docs/`, git log around `00fc989`/`c747dc4`.

This task did not run under the five-file AI Orchestrator contract (no `request.json`/`task.md`/`progress.json`/`result.json` were provided for this session) — it was executed directly against the task description given in chat, per `02_SOFTWARE_IMPLEMENTATION_EXECUTION.md`.

## Final status: SUCCESS (one physical check remains, see §7)

Both requested improvements are implemented, tested, built, and verified. No functional regression to right-click/gesture/tray/hook behavior was made (mouse_hook.rs and lib.rs gesture logic were not touched). `ACTION_LABEL_OVERLAY_ENABLED = false` is unchanged.

## 1. Repository-level distribution layout (Part A)

**Previous deep artifact paths** (unchanged, still Tauri's internal output):
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/GestureHotkeyApp.exe`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/bundle/nsis/GestureHotkeyApp_0.1.0_x64-setup.exe`

**New public distribution path:**
```
C:\GitHub\open-mouse-gesture\dist\windows\
  OpenMouseGesture-x64.exe
  OpenMouseGesture-Setup-x64.exe
  SHA256SUMS.txt
  build-info.json
```
Names are stable/predictable (no version suffix) so the command is always safe to re-run and the location never changes; the version is recorded inside `build-info.json` instead.

**Export command:** `npm run dist:windows` (run from the repo root, **after** `npm run tauri build` inside the source dir). Implementation:
- `scripts/dist-windows-lib.mjs` — pure helpers (`sha256File`, `formatSha256SumsFile`, `buildMetadata`, `resolveInstallerFileName`), unit-testable without touching the filesystem beyond a temp dir.
- `scripts/dist-windows.mjs` — CLI orchestrator: locates `GestureHotkeyApp.exe` and the NSIS `*_x64-setup.exe` under the Tauri release/bundle output (reading `mainBinaryName`/`version`/`productName` from `tauri.conf.json`), fails with a clear message telling the user to run `npm run tauri build` first if artifacts are missing, deletes and recreates only `dist/windows/` (never touches `target/`), copies both artifacts under the stable names above, verifies the copied bytes hash identically to the source before writing anything, writes `SHA256SUMS.txt` (sha256sum-compatible format) and `build-info.json`.
- Root `package.json` (new — none existed before) exposes `dist:windows` and `test:dist`.
- `dist/README.md` documents the directory and command so both are discoverable after a fresh clone; `dist/windows/` itself is gitignored (`.gitignore` gained `dist/windows/`) so generated `.exe`/hash/metadata files are never committed — only `dist/README.md` is tracked.
- `README.md` (repo root) now has a "配布物 (dist/windows/)" section with the exact two-step process and file list.

**SHA-256 comparison results** (independently recomputed with `sha256sum` outside the script, not just trusted from script output):
```
8d793d65ac139d1dbdbd26014a015842a16c984a0470ce715012af703bca3227  GestureHotkeyApp.exe                        (target/release)
8d793d65ac139d1dbdbd26014a015842a16c984a0470ce715012af703bca3227  dist/windows/OpenMouseGesture-x64.exe        (match)
5fc8160c824a960813ceeccd76ac0bbcacf525a123293d003895eccf3811bce8  GestureHotkeyApp_0.1.0_x64-setup.exe        (bundle/nsis)
5fc8160c824a960813ceeccd76ac0bbcacf525a123293d003895eccf3811bce8  dist/windows/OpenMouseGesture-Setup-x64.exe  (match)
```

**build-info.json example (actual, from this run):**
```json
{
  "productName": "GestureHotkeyApp",
  "version": "0.1.0",
  "buildTimestamp": "2026-07-18T00:58:32.887Z",
  "gitCommit": "c747dc4892b159d49124e1ee9c81afa341856531",
  "artifacts": [
    { "name": "OpenMouseGesture-x64.exe", "sha256": "8d793d65...bca3227", "sizeBytes": 12838400 },
    { "name": "OpenMouseGesture-Setup-x64.exe", "sha256": "5fc8160c...11bce8", "sizeBytes": 2799808 }
  ]
}
```
Note: `tauri.conf.json`'s `version` field is `0.1.0` while `Cargo.toml`'s package version is `1.0.1` — this discrepancy pre-exists in the repo (not introduced or fixed by this task); `build-info.json` faithfully reports the Tauri-facing version since that's what the installer itself embeds. Flagging for awareness, not treating as in-scope to fix.

**Stale-artifact safety:** verified by dropping a marker file into `dist/windows/` and re-running `npm run dist:windows` — the marker was removed and only the four expected files remained. The script only ever deletes inside `dist/windows/`.

## 2. Trajectory visual refinement (Part B)

File: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs` (native Win32 layered-window GDI renderer — this is what actually draws on screen during a live gesture; it is separate from `GestureCanvas.tsx`, which is only the in-app gesture-pattern editor and was not touched).

**Before:** single `CreatePen(PS_SOLID, 3, color)` stroke, drawn once, then a post-pass forced every non-black pixel to alpha=255 and every black pixel to alpha=0 (`fix_alpha_channel`) — i.e. a flat, fully opaque, single-width, square-jointed line with no translucency possible at all (alpha was always binary).

**After:** three-layer rounded stroke, each layer drawn with `ExtCreatePen(PS_GEOMETRIC | PS_SOLID | PS_ENDCAP_ROUND | PS_JOIN_ROUND, width, ...)` (round caps/joins, requires geometric pens — cosmetic pens only support width 1):

| Layer | Width | Alpha | Color |
|---|---|---|---|
| Glow (outer) | 16px (vs old 3px) | 60/255 (~24%) | base color, darkened 10% toward black |
| Body | 9px (**3x** the old 3px width) | 175/255 (~69%) | base color as configured |
| Core (highlight) | 4px | 235/255 (~92%) | base color, lightened 45% toward white |

Each layer is rasterized into its own cached, window-sized offscreen DIB (resized only when the trajectory's bounding box changes size — not per mouse event) so per-layer pixel coverage can be read back via `GetDIBits`. The three coverage masks are then composited into the final ARGB buffer with `core > body > glow` priority per pixel, and — critically — the output is **premultiplied alpha** (`out.rgb = color.rgb * alpha/255`), which `UpdateLayeredWindow`'s `ULW_ALPHA`/`AC_SRC_ALPHA` blend mode requires for correct partial-opacity compositing against the desktop; the old binary-alpha scheme happened to work without premultiplication only because alpha was always 0 or 255.

Colors are derived from the user's existing per-trigger color setting (`triggerAColor`/`B`/`C`, e.g. default `#FF4D4F`) via `trajectory_renderer::set_active_color`, unchanged call site — no hardcoded color was introduced, and users keep their own configured trail color, now rendered as a glow/body/core gradient of that color instead of a flat fill.

**Performance considerations:**
- The offscreen mask DCs/DIBs are cached and only reallocated when the trajectory bounding box size changes (mirrors the existing `MEMORY_DC` resize pattern) — no per-mouse-event allocation.
- Rendering is still gated by the existing 16ms (`MIN_FRAME_INTERVAL_MS`) throttle in `append_trajectory_point`/`update_trajectory`, unchanged.
- The compose loop only iterates the trajectory's bounding-box region (same bounded-region approach the old `fix_alpha_channel` used), not the full screen.
- `BOUNDS_MARGIN` was increased from 16px to 26px (`GLOW_WIDTH + 10`) so the wider glow layer isn't clipped at the window edge.

**Tests added** (`#[cfg(test)] mod tests` in `trajectory_renderer.rs`, run via `cargo test`):
- `rgb_from_colorref_roundtrips_hex` — color round-trip through the existing `set_active_color` packing.
- `derive_shades_core_is_brighter_than_body` — core lighter than body, glow not brighter than body.
- `layer_widths_are_at_least_three_times_old_line_width` — body ≥ 3× the old 3px width, glow > body > core.
- `layer_alphas_are_within_translucent_targets` — glow < body < core, and body/core alphas fall within the spec's 0.55–0.8 / 0.8–1.0 targets (175/255≈0.69, 235/255≈0.92).

`ACTION_LABEL_OVERLAY_ENABLED` in `lib.rs` was not touched (still `false`); the existing `trajectory_enabled_toggle_is_independent_of_action_label_overlay_flag` and `show_action_label_for_action_is_a_no_op_when_overlay_disabled` tests in `lib.rs` still pass unchanged, confirming the two systems remain independent.

## 3. Tests and build results

- `cargo test` (in `src-tauri/`): **26/26 passed** (22 pre-existing + 4 new trajectory tests). No failures, no regressions. `mouse_hook::tests::*` (right-click replay, trigger slot resolution, gesture session transitions, hook cleanup) all pass unchanged, confirming no regression to the `00fc989`/`c747dc4` fixes.
- `npm run build` (frontend, tsc + vite): succeeded, `dist/` produced (238KB JS, 24KB CSS).
- `npm run tauri build`: succeeded end-to-end — release Rust build, NSIS patch, `makensis` bundling. Produced `GestureHotkeyApp.exe` and `GestureHotkeyApp_0.1.0_x64-setup.exe`.
- `npm run test:dist` (root, `node --test scripts/*.test.mjs`): **7/7 passed** (sha256 hashing, SHA256SUMS formatting, build-info metadata shape incl. null-commit fallback, installer-filename resolution incl. none/multiple-found error paths).
- `npm run dist:windows`: succeeded, exported and hash-verified as in §1; re-run confirmed stale-file cleanup.

All warnings seen during `cargo build`/`cargo test` (unused `mut`, non-snake-case identifiers in `config.rs`/`lib.rs`) are pre-existing and unrelated to this change; only 2 of the ~15 warnings (`unused_mut` on lines 304/383 of the new `trajectory_renderer.rs`) were introduced by this change and are cosmetic (no behavior impact) — left as-is since `CLAUDE.md`'s minimal-diff/no-scope-creep rule argues against unrelated cleanup, but noting them here for transparency.

## 4. Installer / EXE artifact details

- `dist/windows/OpenMouseGesture-x64.exe` — 12,838,400 bytes, sha256 `8d793d65ac139d1dbdbd26014a015842a16c984a0470ce715012af703bca3227`
- `dist/windows/OpenMouseGesture-Setup-x64.exe` — 2,799,808 bytes, sha256 `5fc8160c824a960813ceeccd76ac0bbcacf525a123293d003895eccf3811bce8`
- Both built from commit `c747dc4892b159d49124e1ee9c81afa341856531` **plus** the working-tree trajectory renderer change in this session (the export was run after the new build, so the hashes reflect the new trajectory code, not the pre-existing commit alone).

## 5. Changed files (this session)

```
M  .gitignore                                                          (ignore dist/windows/)
M  README.md                                                           (dist/windows/ + command docs, JP)
A  dist/README.md
A  package.json                                                        (new root package.json; dist:windows/test:dist scripts)
A  scripts/dist-windows-lib.mjs
A  scripts/dist-windows-lib.test.mjs
A  scripts/dist-windows.mjs
M  source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs
```
Not staged/committed (pre-existing, no actual content diff beyond line-ending metadata noise, unrelated to this task): `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/Cargo.toml`, `.../license.html`.
Not committed (generated, gitignored): `dist/windows/*`.

## 6. Git

Commit SHA: see final `git log -1` after this report is written (this report is committed alongside the code in the same commit per the user's earlier "preserve unrelated work" / "only commit what's relevant" instruction).
Push destination: `origin/main`, ordinary fast-forward push (no force).
Final worktree state: clean except pre-existing unrelated `Cargo.toml`/`license.html` line-ending noise (left untouched) and the gitignored `dist/windows/` output.

## 7. Remaining physical verification (exactly one check)

The rebuilt app was **not launched or installed** in this session: `GestureHotkeyApp.exe` was confirmed not currently running, and the NSIS installer is `perMachine` (requires an interactive UAC elevation prompt only you can approve — same constraint noted in the prior `00fc989`/`c747dc4` session's report). Installing or running the new global mouse/keyboard hook automatically, without your explicit action, would affect your live desktop session, so it was left for you.

**What to do:** run `dist\windows\OpenMouseGesture-Setup-x64.exe` (or launch `dist\windows\OpenMouseGesture-x64.exe` directly for a quick smoke test without installing), then draw one gesture (e.g. hold Mouse Right and drag) and confirm:
1. The trajectory is visibly thicker with a soft glow/translucent look (not a flat thin red line), in your currently configured trigger color.
2. Drawing still feels smooth with no perceptible lag.
3. Right-click context menus, middle-click gestures, and keyboard triggers still behave as before (no regression expected — hook/gesture code was not touched — but worth a quick confirmation since this is the first build since `c747dc4`).

## User decision required: none

## Final status: SUCCESS, pending the one physical check in §7.
