# report.md — Open Mouse Gesture: Trajectory Design B / Jitter Fix / Legacy Settings Import

## 対象

Working directory: `C:\GitHub\open-mouse-gesture`
Primary source: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
実行: Claude Code CLI, Sonnet, medium effort。

## A. トラジェクトリ デザインB（濃い赤コア + 柔らかい外側グロー）

対象ファイル: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`

### 変更前

`derive_shades()` はコアをベース色から**白へ45%寄せて**明るくしていた
(`lerp_u8(base, 255, 0.45)`)。結果としてコアが白っぽく薄く見え、
「too pale」というユーザー指摘の直接原因になっていた。

- `CORE_ALPHA = 235`, `BODY_ALPHA = 175`, `GLOW_ALPHA = 60`
- core = ベース色を白へ45%寄せた色（薄い）
- body = ベース色そのまま
- glow = ベース色を黒へ10%寄せた色

### 変更後（デザインB）

- **コア**: ベース色を**黒へ12%だけ寄せる**（白へ寄せない）。彩度・強さを保ち、
  むしろ密度感を出す。`CORE_ALPHA = 255`（ほぼ完全不透明）。
- **ボディ**: ベース色そのまま、`BODY_ALPHA = 150`（半透明で太さを支える）。
- **グロー**: ベース色を白へ20%寄せた柔らかい色、`GLOW_ALPHA = 55`（低不透明度で外側へ自然にフェード）。
- 太さ（`CORE_WIDTH=4`, `BODY_WIDTH=9`, `GLOW_WIDTH=16`）はユーザー確認済みの現状を維持し、変更していない。
- 丸線端/丸結合ペン(`PS_ENDCAP_ROUND | PS_JOIN_ROUND`)、プリマルチプライ済みARGB合成のロジックはそのまま維持。
- トリガー色は `set_active_color()` 経由で `ACTIVE_LINE_COLOR` に反映され、`derive_shades()` が都度そこから3色を導出する仕組みは変更していない（トリガーごとの色設定を維持）。

## B. ジッター（トレンブリング）の原因診断と修正

### 根本原因

`TRAJECTORY_POINTS`（点列）と `TRAJECTORY_BOUNDS`（ウィンドウ位置/オフセット計算用の矩形）が
**別々の `Mutex`** で管理されていた。`window_proc` の `WM_UPDATE_TRAJECTORY` ハンドラは

```rust
let bounds = TRAJECTORY_BOUNDS.lock().unwrap();
let points = TRAJECTORY_POINTS.lock().unwrap();
(bounds.clone(), points.clone())
```

の順で2回別々にロックを取得してスナップショットしていた。一方、`append_trajectory_point()` /
`update_trajectory()` は点列とバウンディングを**別のブロックで逐次更新**していたため、
レンダースレッドが `TRAJECTORY_BOUNDS` を読んだ直後・`TRAJECTORY_POINTS` を読む前に、
マウスフックスレッドが新しい点を `TRAJECTORY_POINTS` に追加すると、
「新しい点列」と「古いバウンディング(=古いウィンドウ位置・描画オフセット)」の
**不整合な組み合わせ**が1フレームだけ発生する。ウィンドウ位置と描画オフセットは
バウンディングから導出されるため、この不整合フレームでは既に描画済みの経路全体が
一時的にずれて見え、次のフレームで正しい位置に戻る——これが連続すると
「軌跡が震える」ように知覚される。レイヤー化(グロー/ボディ/コア3層)自体は
同じ関数内で同じスナップショットを使っており独立してずれてはいなかったが、
フレーム単位で全体が不整合になっていた。

該当箇所（修正前）: `trajectory_renderer.rs` の `WM_UPDATE_TRAJECTORY` ハンドラ、
`update_trajectory()`, `append_trajectory_point()`。

### 修正内容

1. `TRAJECTORY_POINTS` / `IS_VISIBLE` / `TRAJECTORY_BOUNDS` / `PREVIOUS_BOUNDS`（未使用の書き込み専用フィールドだったため削除）を、
   単一の `TRAJECTORY_STATE: Mutex<TrajectoryState { points, visible }>` に統合。
2. 純粋関数 `compute_bounds(points: &[(i32,i32)]) -> Option<RECT>` を新設し、
   バウンディング矩形の計算式を1箇所に集約。
3. `window_proc` は `TRAJECTORY_STATE` を**1回のロックで**スナップショットし、
   その同じ点列から `compute_bounds()` でバウンディングを都度再計算する。
   ウィンドウ配置・DIBクリッピング・3層すべての描画オフセットが、
   常に同一の点列スナップショット・同一の計算式に由来するため、
   点列とバウンディングが異なるタイミングを参照することが構造的に不可能になった。
4. `update_trajectory()` / `append_trajectory_point()` / `clear_trajectory_display()` を
   単一ロックでの点列・可視状態更新に書き換え。

これは「過剰な平滑化でごまかす」のではなく、競合状態そのものを排除する修正であり、
要求どおり「既に描画された経路は視覚的に固定される」「全レイヤーが同一の安定したジオメトリを使う」を満たす。

### 安定性の検証（自動）

`compute_bounds()` に対する単体テストを追加（`cargo test` で実行、全て pass）:

- `compute_bounds_is_none_for_empty_points`
- `compute_bounds_is_deterministic_for_same_points`
- `compute_bounds_is_invariant_to_point_insertion_order` — 同じ点集合なら到着順に関わらず同一バウンディングになることを保証（ロック統合の効果を裏付ける）
- `compute_bounds_matches_margin_formula`
- `compute_bounds_grows_monotonically_as_points_are_appended` — バウンディングは単調拡大のみで縮小しない = 既存の描画済み点が再クリップされない

**未検証事項**: 実機での物理マウス操作による視覚確認（震えが消えたこと）は本セッションでは実施していない。ロジック上のレースコンディションは特定・除去済みで、静的解析・自動テストでは裏付けられているが、GDI/レイヤードウィンドウの実際の描画結果を目視する物理確認は未実施。

## C. 過去のマウスジェスチャー設定インポート成果物

### 検索範囲

`legacy`, `old`, `backup`, `migration`, `import`, `converted`, `preset`, `profile`, `settings`,
`config`, `gesture`, `mapping` などをファイル名・内容の両方で検索。`docs/`, `artifacts/`,
`legacy/GestureHotkeyApp-Wpf/`, リポジトリ全体を対象にした。

### 候補一覧

1. **`artifacts/GestureHotkeyApp-settings-batch1.gha.json`** — `formatVersion: 1`, `appName: "GestureHotkeyApp"` を持つ
   `SettingsBundle` 形式のJSON。`config`（trajectory, ignore_exe, triggerA/B/C, triggerAColor/B/C, groups, actions 20件）と
   `gestures`（7ジェスチャーテンプレート）を含む。現行 Rust の `SettingsBundle` / `Config` / `GestureTemplate` 構造体
   (`src-tauri/src/config.rs`) と**フィールドが完全一致**しており、`docs/settings-export-import-plan.md` に記載の
   `GestureHotkeyApp-settings.gha.json` という既定ファイル名パターンとも一致する。
2. `legacy/GestureHotkeyApp-Wpf/` — .NET8/WPF による旧試作アプリのソース一式。
   `Services/JsonConfigurationService.cs` はこのWPF版自身の設定保存ロジックであり、
   Tauri版の `SettingsBundle` 形式とは異なるスキーマ。インポート用に整形済みの成果物ではなく、参照用ソースコード。
3. `release-v1.0.1/config.json`, `release-v1.0.1/gestures.json` — 旧リリース配置の設定・ジェスチャー(単体ファイル、`SettingsBundle` ラッパーなし)。
4. `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/config/config.json`, `.../config/gestures.json` — 開発用デフォルト設定
   （`config.json` は triggerA/B/C や groups を含まない古い形式で、`gestures.json` は候補1の `gestures` と同一内容）。

### 選定と理由

**候補1 `artifacts/GestureHotkeyApp-settings-batch1.gha.json` を採用。**
理由: 現行アプリの `SettingsBundle` スキーマと1対1で対応し、変換なしに構造がそのまま使える。
`docs/settings-export-import-plan.md` が定めるバンドル形式・既定ファイル名規則とも一致しており、
「今回の作業のために用意された」成果物として最も蓋然性が高い。WPF版ソースはスキーマが異なりインポート対象として不適当。

### 適用結果

- **アクティブ設定の場所（実際にインストール済みアプリが使用する場所）**:
  `%APPDATA%\GestureHotkeyApp\config.json`, `%APPDATA%\GestureHotkeyApp\gestures.json`
  （`ConfigManager::new()` がリリースビルドで `dirs::config_dir()/GestureHotkeyApp` を使用するため。この環境では実際に存在を確認済み。）
- **バックアップ先**: `%APPDATA%\GestureHotkeyApp\backup-20260718-102227\config.json`,
  `%APPDATA%\GestureHotkeyApp\backup-20260718-102227\gestures.json`
  （置き換え前の実機検証済み設定を複製。triggerA=mouse:right, triggerB=mouse:middle, triggerC=mouse:x1 のアクション5件を含む。）
- **適用した内容**:
  - `config.json`: 候補1の `config` をそのまま採用し、`triggerA/B/C` のみ `"right"/"middle"/"x1"` →
    `Config::normalized()` と同じ変換規則で `"mouse:right"/"mouse:middle"/"mouse:x1"` に正規化して書き込み。
    `triggerAColor/B/C`（`#FF4D4F` / `#4C8DFF` / `#22A06B`）はバックアップ前の値と同一のため変更なし。
    `groups` 4件、`actions` 21件（gesture 20件 + trigger_slot A/B/C混在、window_operation 含む）を反映。
  - `gestures.json`: 候補1の `gestures`（7テンプレート: 左/右/下/上/下→右/上→下/G）を採用。
    これは開発用デフォルト `config/gestures.json` と内容が完全一致しており、`diff` で確認済み。
- **未マッピング項目**: なし。候補1のフィールドは現行スキーマの全フィールド（`trajectory`, `ignore_exe`,
  `triggerA/B/C`, `triggerAColor/B/C`, `groups`, `actions[].{name, group_id, trigger_type, trigger_slot,
  gesture, action_type, keystroke, modifiers, command, url, operation, ignore_exe}`, `gestures[].{name, points}`）に
  1対1で対応し、変換不能・意味不明な項目は無かった。
- **検証**: PowerShell (`ConvertFrom-Json` + 手動検証、`-Encoding UTF8` 指定) で、全 `actions` の
  `group_id` が既知グループに一致、`trigger_slot` が A/B/C のいずれか、`action_type` が
  `keystroke|command|url|window_operation` のいずれか、`triggerA/B/C` が `mouse:`/`key:` 正規化済みであることを確認（`actions=21 groups=4` / VALID）。
  Rust 側の `Config::validate()` / `GestureTemplate::validate()` 相当のロジックを手動で再現した確認であり、
  実際にアプリを起動して `ConfigManager::load_config()` を通した動作確認は未実施（下記「未検証事項」参照）。
- **トリガー保持**: `triggerA/B/C` は物理検証済みのバインディング（right/middle/x1）と同一の値を維持しており、
  今回のトリガー機能（右クリック/右ドラッグ/ミドルクリック/キーボードトリガー）の回帰は発生しない設計。

## D. 回帰要件

- 右クリックのコンテキストメニュー・パススルー、右ドラッグジェスチャー、Middle/X1/X2/キーボードトリガー、
  Trigger A/B/C 永続化、トレイ動作、フックのクリーンアップに関わるコードは**一切変更していない**
  （変更ファイルは `trajectory_renderer.rs` のみ）。
- `ACTION_LABEL_OVERLAY_ENABLED = false` は `src-tauri/src/lib.rs:31` のまま不変（変更なし、grep で確認済み）。
- `dist/windows/` は再生成し、既存の運用フロー（`npm run tauri build` → `npm run dist:windows`）で問題なく生成できた。

## E. テスト・ビルド結果

| 項目 | 結果 |
|---|---|
| `cargo test --lib trajectory_renderer`（フォーカステスト） | **PASS** 10/10 |
| `cargo test`（フル） | **PASS** 32/32、0 failed |
| `npm run build`（フロントエンド, tsc + vite） | **PASS**（`dist/index.html` ほか生成、606ms） |
| `npm run tauri build`（リリースビルド + NSISインストーラー） | **PASS**（警告は既存のnon-snake-case警告のみ、新規エラーなし） |
| `npm run dist:windows`（配布物エクスポート） | **PASS** |

## F. `dist/windows/` 成果物

```
OpenMouseGesture-x64.exe          12,832,768 bytes
  sha256=f518a7c65439cd144ceb38d611522454eafa02a4a7cb2c92b318f480493473e2
OpenMouseGesture-Setup-x64.exe     2,796,585 bytes
  sha256=8d1f0903f38db9880f3e718c31246cadf748fd0c75964b5a4d7d3f39f9ab5b5b
```

`SHA256SUMS.txt` の内容と実ファイルのハッシュ一致を `sha256sum -c` で確認済み（両方 `OK`）。
`build-info.json`: `version=0.1.0`, `gitCommit=ce3d58a7c6e12bc142f1753bae9db871f14c636d`
（このコミットは配布物ビルド時点の直近コミットで、今回のソース変更コミット `8b47b41` はビルド後に作成したため
`build-info.json` の `gitCommit` には未反映。再現性に影響する場合は `npm run dist:windows` を再実行してコミットSHAを更新可能——安全な再実行なので必要なら実施できるが、今回の指示範囲では既存の運用フローどおり1回の生成で完了とした。）

## インストール状況

`dist/windows/OpenMouseGesture-Setup-x64.exe /S` によるサイレントインストールを試行したが、
`Permission denied` で失敗（NSISインストーラーが管理者権限を要求するため、非対話シェルからは実行不可）。
UAC昇格が唯一のブロッカーであり、これ以上は安全に自動化できないため停止した。

**残っている物理確認（1件のみ）**:
`dist\windows\OpenMouseGesture-Setup-x64.exe` を手動でダブルクリックしてUACを承認しインストールし、
実機でジェスチャー軌跡が「濃い赤コア + 柔らかいグロー」で表示され、震えが見られないことを目視確認してください。

## 変更ファイル

- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`（唯一のソース変更）
- `%APPDATA%\GestureHotkeyApp\config.json`, `%APPDATA%\GestureHotkeyApp\gestures.json`（アプリ実行時設定、リポジトリ外・Git管理対象外）
- `%APPDATA%\GestureHotkeyApp\backup-20260718-102227\*`（バックアップ、リポジトリ外）
- `dist/windows/*`（Git管理対象外、`dist/README.md` の方針どおり）
- `report.md`（本ファイル、上書き更新。前回セッション分の内容は本レポートに置き換え）

pre-existing だった `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/Cargo.toml` と
`.../license.html` の未ステージ変更（行末コード変換の警告のみで実質差分なし）は、本タスクと無関係のため
**変更もコミットもしていない**（作業開始時点で既に working copy に存在していた状態を保持）。

## Git

- コミットSHA: `8b47b41bbb5f397576ca134d0bda53ddca6b9f71`
- Push先: `origin/main`（`https://github.com/ordinal-aitech/open-mouse-gesture.git`, `ce3d58a..8b47b41 main -> main`）
- 最終 `git status`: `Your branch is up to date with 'origin/main'.` ステージ済み変更なし。
  未ステージで `Cargo.toml` / `license.html` の行末警告のみ残存（本タスク開始前からの既存状態、意図的に保持）。

## 未検証事項

- 実機物理操作による、震えが解消されたことの目視確認（上記「残っている物理確認」参照）。
- 実機物理操作による、コアの濃さ/グローの柔らかさがデザインB意図どおり見えることの目視確認。
- 新しい `config.json`/`gestures.json` を実際にインストール済みアプリで読み込ませ、`ConfigManager::load_config()` が
  エラーなく起動することの実行時確認（構造的な検証はPowerShellで実施済みだが、アプリ自体の起動確認は未実施）。
- `dist/windows/build-info.json` の `gitCommit` は今回のソース変更コミット以前のものであり、
  完全な再現性が必要な場合は `npm run dist:windows` の再実行が必要（安全な操作、未実施）。

## 判断待ち事項

`User decision required: none`

## 最終ステータス

**Paused for one physical action**: 自動化可能な範囲（デザインB実装、ジッター根本原因の特定と修正、
テスト、ビルド、配布物生成、レガシー設定の発見・バックアップ・適用、コミット・プッシュ）はすべて完了。
UACによる手動インストール承認のみ、ユーザーの物理操作が必要。
