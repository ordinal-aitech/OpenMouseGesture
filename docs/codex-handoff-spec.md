# Codex 引き継ぎ仕様書

## 1. この文書の目的

この文書は、別 PC 上の新しい Codex / ChatGPT が `C:\GitHub\OpenMouseGesture` リポジトリを開いたときに、
現状の前提・実装状態・次に進めるべき作業を短時間で把握できるようにするための引き継ぎ仕様書です。

この文書を読む側は、まずこのファイルを正として状況把握し、その後に必要な詳細だけ各設計書やコードへ掘り下げる想定です。

## 2. 現在の作業対象

### 正式名称
- アプリ名: `GestureHotkeyApp`
- 実行ファイル名: `GestureHotkeyApp.exe`
- インストーラー名: `GestureHotkeyApp-setup.exe` を目標

### 現在の改修対象
- ベース: OpenMouseGesture
- 主作業ディレクトリ:
  - `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`

### 旧試作の扱い
- 旧 WPF 試作は参照用として隔離済み
  - `legacy/GestureHotkeyApp-Wpf/`
- 現在の主線は WPF 試作ではなく、OpenMouseGesture 改造版

## 3. リポジトリ構成

- `source-v1.0.1/`
  - OpenMouseGesture 改造版ソース
- `release-v1.0.1/`
  - 元の配布物
- `docs/`
  - 要件、改修設計、進捗、テスト結果、スクリーンショット
- `artifacts/`
  - ビルド補助スクリプト、アイコン確認用画像など
- `legacy/GestureHotkeyApp-Wpf/`
  - 旧 .NET 8 / WPF 試作

## 4. 今回のアプリ方針

### 目的
- Windows 上でマウスジェスチャーを認識し、割り当てた Hotkey を送信する

### 維持したい制約
- 既存 UI はできるだけ流用
- `trigger_slot + gesture` 単位で action を持つ
- Trigger A / B / C の 3 系統を持つ
- Trigger ごとに開始ボタンを個別設定できる
- Trigger ごとに軌跡色を個別設定できる
- `XBUTTON1 / XBUTTON2` を trigger 候補に含める

### 今は入れない方針
- 全面 UI 作り直し
- action label overlay の再有効化
  - 現時点では意図的に無効化中
- 無闇な機能追加

## 5. 現在の実装状態

### 実装済み
- `GestureHotkeyApp` への名称統一
- Tauri / Rust / React ベースへの改修
- Trigger A / B / C
- Trigger ボタン候補
  - `right`
  - `middle`
  - `x1`
  - `x2`
- Trigger ごとの色設定
- `trigger_slot + gesture` ベースの action 識別
- export / import の settings bundle
- grouped action list
- group を独立単位にしたデータ構造
  - `Config.groups`
  - `Action.group_id`
- group header での折りたたみ
- group 単位の action 追加導線

### 意図的に無効化中
- action label overlay
  - Rust 側 `ACTION_LABEL_OVERLAY_ENABLED = false`
  - フリーズ、軌跡残り、操作阻害を避けるため一旦停止

### まだ未確認または未完了
- group 名のインライン編集の実操作確認
- 新規 group 追加直後の rename UX
- action を別 group へ移動する UI
- export / import の別 PC での end-to-end 実運用確認
- `GestureHotkeyApp-setup.exe` という最終ファイル名への統一確認
  - 現時点で確認済み生成物は `GestureHotkeyApp_0.1.0_x64-setup.exe`

## 6. 重要データモデル

### TypeScript 側
- `src/types/index.ts`

重要項目:
- `TriggerSlot = "A" | "B" | "C"`
- `GestureTriggerButton = "right" | "middle" | "x1" | "x2"`
- `ActionGroup`
  - `id`
  - `name`
- `Action`
  - `group_id`
  - `trigger_slot`
- `Config`
  - `triggerA/B/C`
  - `triggerAColor/B/CColor`
  - `groups`
  - `actions`

### Rust 側
- `src-tauri/src/config.rs`

重要点:
- 旧 `Action.group` 文字列を migration 用に受ける
- 正規化時に `groups + group_id` へ移行
- `group` は保存時に出さない

## 7. 重要ファイル

### フロント
- `src/components/actions/ActionsTab.tsx`
- `src/components/actions/ActionList.tsx`
- `src/components/actions/ActionEditor.tsx`
- `src/components/actions/ActionList.css`
- `src/components/settings/SettingsTab.tsx`
- `src/store/useStore.ts`
- `src/types/index.ts`
- `src/api/commands.ts`

### バックエンド
- `src-tauri/src/config.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/mouse_hook.rs`
- `src-tauri/src/trajectory_renderer.rs`
- `src-tauri/src/action_label_overlay.rs`
- `src-tauri/tauri.conf.json`

### 設計・記録
- `docs/openmousegesture-mod-plan.md`
- `docs/action-list-grouping-plan.md`
- `docs/settings-export-import-plan.md`
- `docs/windows-installer-plan.md`
- `docs/app-branding-plan.md`
- `docs/progress-log.md`
- `docs/test-result-summary.md`

## 8. 既知の仕様判断

### action grouping
- `ActionEditor` に group 名入力欄は置かない
- group 名は group 側で管理
- action は `group_id` 参照

### action label overlay
- 現時点では「直す」より「無効化維持」が正
- 再導入するなら、
  - 入力フックを重くしない
  - 軌跡描画を壊さない
  - カーソル移動を阻害しない
 ことが前提

### docs の扱い
- 一部ドキュメントは PowerShell 上で日本語が文字化けして見えることがある
- 内容確認や編集は UTF-8 を扱えるエディタで見る前提が安全

## 9. ビルド環境

別 PC で最低限必要なもの:
- Node.js LTS
- npm
- Rust / cargo
- Visual Studio 2022 Build Tools
  - MSVC toolchain
- Git
- NSIS
- `cargo-about`

参考:
- `docs/build-environment-setup.md`
- `artifacts/run-tauri-build.cmd`

## 10. ビルド手順

主対象ディレクトリ:
- `C:\GitHub\OpenMouseGesture\source-v1.0.1\7-rate-OpenMouseGesture-b8f5357`

### フロント build
```powershell
cd C:\GitHub\OpenMouseGesture\source-v1.0.1\7-rate-OpenMouseGesture-b8f5357
npm install
npm run build
```

### Tauri build
```powershell
cd C:\GitHub\OpenMouseGesture\source-v1.0.1\7-rate-OpenMouseGesture-b8f5357
npm run tauri build
```

または補助スクリプト:
```powershell
cmd /c C:\GitHub\OpenMouseGesture\artifacts\run-tauri-build.cmd
```

## 11. 確認済み成果物

過去の確認では以下が生成済み:
- `src-tauri/target/release/GestureHotkeyApp.exe`
- `src-tauri/target/release/bundle/nsis/GestureHotkeyApp_0.1.0_x64-setup.exe`

## 12. 次の優先順

新しい Codex が続行する場合のおすすめ順:

1. `source-v1.0.1/...` を基準に `npm run build` / `npm run tauri build` が通るか再確認
2. `Config.groups + Action.group_id` 周りの UI 実動作を確認
3. group 名編集の UX を詰める
4. action の group 移動導線を設計・実装
5. export / import を別 PC 想定で再確認
6. NSIS 出力名と Program Files 配置の最終確認

## 13. 新しい Codex への引き継ぎメモ

次の Codex はまず以下を読むと早いです:

1. この `docs/codex-handoff-spec.md`
2. `docs/progress-log.md`
3. `docs/openmousegesture-mod-plan.md`
4. `docs/test-result-summary.md`

そのうえで、主作業ディレクトリを
`source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
に固定して進めてください。

旧 WPF 試作は `legacy/` に隔離済みなので、現在の主線に混ぜないでください。
