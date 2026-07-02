# 進捗ログ

## 2026-06-23

### 初期 MVP 実装

- `.NET 8 + WPF + Win32 P/Invoke` ベースで常駐型マウスジェスチャー MVP を実装
- トレイアイコン、トリガー A/B、グローバル設定、アプリ別プロファイル、Hotkey 送信、JSON 保存を作成
- 実装計画と README を整備

## 2026-06-26

### 軌跡描画と入力系の改善

- 軌跡描画の座標系補正を追加
- U / D 方向判定の逆転を修正
- Right Trigger に移動しきい値を導入
- trail session 管理を追加し、開始・更新・完了・クリアの流れを整理

## 2026-06-27

### OpenMouseGesture ベースへの移行調査

- GitHub Release `v1.0.1` の実行バイナリとソースを `external/OpenMouseGesture` 配下へ配置
- 既存 UI は React + TypeScript + Zustand、バックエンドは Tauri 2 + Rust + windows-rs であることを確認
- 現状の gesture 開始は実質 Right 固定で、3 trigger 化には `mouse_hook.rs`、`config.rs`、`SettingsTab.tsx`、`trajectory_renderer.rs` の改修が必要と判断

### 3 トリガー化と SettingsTab 拡張方針

- Trigger A / B / C の 3 slot を導入する設計案を作成
- 開始ボタン候補を `Right / Middle / XBUTTON1 / XBUTTON2` とする方針を整理
- Trigger ごとの軌跡色設定を `SettingsTab` に追加する案を整理
- `trigger_slot + gesture` 単位で別 Hotkey を持つデータモデル案を整理

### アクションラベルオーバーレイ

- 認識済みジェスチャーの Hotkey / アクション名を表示する補助オーバーレイ案を整理
- `action_label_overlay.rs` と `mouse_hook.rs` の拡張ポイントを確認

### ブランド名と Windows 配布方針の確定

- 正式名称を `GestureHotkeyApp` に統一する方針を反映
- Tauri の `productName`、`mainBinaryName`、Windows インストーラー名、Program Files 配下フォルダ名を揃える設計を追加
- Windows インストーラーは NSIS の `-setup.exe` 形式を採用する前提で整理
- `perMachine` インストールで `C:\Program Files\GestureHotkeyApp\` を既定にする方針を追加
- スタートメニューとショートカットに同一アイコンを反映する方針を追加

### アイコン素材と設定反映

- `src-tauri/icons` 配下に `GestureHotkeyApp` 用の app / installer / uninstaller アイコンを生成
- `tauri.conf.json` を更新し、`productName`、`identifier`、`mainBinaryName`、NSIS bundling 設定を反映
- `package.json`、`index.html`、`InfoTab.tsx` の表示名を `GestureHotkeyApp` 基準へ更新
- アイコン確認用プレビューを `artifacts/gesturehotkeyapp-icon-preview.png` として出力

### 3 トリガー化の実装着手

- TypeScript 側へ `TriggerSlot = A/B/C`、`GestureTriggerButton = right/middle/x1/x2` を追加
- `Config` に `triggerA/B/C` と `triggerAColor/B/CColor` を追加
- `Action` に `trigger_slot` を追加
- `src/utils/actionKey.ts` を追加し、`gesture:{slot}:{gesture}` / `wheel:{trigger}` で action key を統一
- `SettingsTab` に `トリガーボタン設定` セクションを追加し、Trigger A/B/C の開始ボタンと軌跡色を編集可能にした
- `ActionsTab` / `ActionList` / `ActionEditor` を `trigger_slot + gesture` 前提へ変更
- Rust 側 `config.rs` を再構成し、migration・fallback・validation を追加
- Rust 側 `lib.rs` で action key 解決と trigger slot ごとの action 解決関数を追加
- Rust 側 `mouse_hook.rs` を trigger slot 設定参照の開始判定へ切り替え
- Rust 側 `trajectory_renderer.rs` に active color 切り替え API を追加

### ビルド環境メモ

- `node` 未導入
- `cargo` 未導入
- `git` 未導入
- そのため、実ビルド・実行確認・NSIS 生成は未実施

### 設定 export / import の追加

- `config.rs` に settings bundle 構造を追加
- `config + gestures` を 1 ファイルへ JSON export する処理を追加
- import 時は bundle を validation / normalize してから保存する構成にした
- `SettingsTab` に `設定をエクスポート` / `設定をインポート` ボタンを追加
- import 後は Config と gesture 一覧を再読込して即時反映する流れを追加
- 仕様メモを `docs/settings-export-import-plan.md` に追加

### 現時点の制約

- この実行環境には `node` と `cargo` が入っていないため、Tauri の実ビルドと NSIS インストーラー生成は未実施
- そのため今回の進捗は、設計更新・設定反映・素材準備までを優先する

## 2026-06-28

### ビルド環境の整備完了

- `Node.js LTS`、`Git`、`rustup / cargo`、`Visual Studio 2022 Build Tools` を導入
- `link.exe` / `cl.exe` を含む MSVC ツールチェーンを確認
- `cargo-about` を追加し、Tauri の `build.rs` 依存も解消
- `artifacts/run-tauri-build.cmd` を追加し、`VsDevCmd.bat` 経由のビルド手順を固定

### ビルド確認

- `npm install` 成功
- `npm run build` 成功
- `npm run tauri build` 成功
- 生成物確認:
  - `src-tauri/target/release/GestureHotkeyApp.exe`
  - `src-tauri/target/release/bundle/nsis/GestureHotkeyApp_0.1.0_x64-setup.exe`
  - `src-tauri/target/release/bundle/nsis/GestureHotkeyApp-setup.exe`

### 起動確認

- `GestureHotkeyApp.exe` を実起動し、プロセス起動を確認
- この時点では「起動すること」までは確認済み
- UI 操作、ジェスチャー入力、Trigger A/B/C 個別動作、export / import end-to-end は未確認

### 設定保存先の修正

- release ビルド時の設定保存先が exe 隣接フォルダだったため、`C:\Program Files\GestureHotkeyApp\` 配置後に通常権限で設定保存できない問題があると判断
- 保存先を `%APPDATA%\GestureHotkeyApp\` に変更
- 旧 release 配置（exe 隣接）の `config.json` / `gestures.json` がある場合は、新保存先へコピー移行する fallback を追加
- 起動後に `%APPDATA%\GestureHotkeyApp\config.json` が生成されることを確認

### 設定 export / import の整合

- `config + gestures` を 1 ファイルの settings bundle として出し入れする Rust 側 API を追加
- `SettingsTab` に `設定をエクスポート` / `設定をインポート` を追加
- 既定ファイル名は `GestureHotkeyApp-settings.gha.json`
- import 時は上書き確認を出し、その後 `Config` と `gesture` 一覧を再読込する
- validation / normalize / fallback を通して保存する構成にした

### 未確認項目

- `SettingsTab` 上で Trigger A / B / C の編集が実 UI で問題なく行えるか
- Trigger ごとの軌跡色が保存 / 再読込されるか
- `trigger_slot + gesture` ごとの別 Hotkey が実動作するか
- `XBUTTON1 / XBUTTON2` の実機入力確認
- ラベルオーバーレイの表示タイミング確認
- export / import の end-to-end 復元確認
- NSIS インストーラーの実インストール、`C:\Program Files\GestureHotkeyApp\` 配置、スタートメニュー反映確認

### アクション一覧 UI 調整

- `ActionList.tsx` / `ActionList.css` を調整
- Trigger 情報の重複表示を整理し、一覧内では trigger 情報を 1 箇所だけにした
- 一覧カードをコンパクト化し、`左: アクション名 / 中: Trigger + ジェスチャー / 右: 実行内容` に寄せた
- gesture プレビューを縮小しつつ、ジェスチャー名テキストも併記する形に変更
- 実画面スクリーンショットを取得:
  - `docs/test-artifacts/action-list-ui-after.png`

### `past` -> `paste` 補正

- 既存保存データに `name = "past"` が残っていることを確認
- `config.rs` の `Action::normalized()` に補正を追加
- 起動後に `%APPDATA%\GestureHotkeyApp\config.json` の保存データが `paste` へ更新されたことを確認

### アクションラベルオーバーレイ追加修正

- `mouse_hook.rs` の認識プレビュー条件を見直し
  - `PREVIEW_MIN_POINTS`: `12 -> 6`
  - `PREVIEW_INTERVAL_MS`: `50 -> 16`
- 同一認識結果でもラベル更新を再送するように変更
- ButtonUp 直前にも強制 preview 更新を走らせるようにして、短いジェスチャーでも表示チャンスが途切れにくい構成にした
- ユーザー実機で「表示されていない」ことが確認されたため、さらに `action_label_overlay.rs` を再修正
  - message 経由更新から direct apply 方式へ変更
  - overlay ready フラグを追加
  - overlay サイズ拡大
  - 下端マージン拡大
  - `SetWindowPos(..., HWND_TOPMOST, ...)` を追加
- この時点で、修正後の物理マウスによる「実際に見えた」確認までは未実施

### アクション一覧 4 列化

- Trigger とジェスチャーを別列へ分離
- 見出しを
  - アクション名
  - トリガー
  - ジェスチャー
  - 内容
  の 4 列に整理
- `ActionsTab.css` 側で editor panel 幅も少し縮め、一覧側の収まりを改善
- スクリーンショット:
  - `docs/test-artifacts/action-list-ui-4col-after.png`

### オーバーレイ一時無効化

- ユーザー実機で、action label overlay が以下の重大不具合を引き起こすことが確認された
  - カーソルがスローモーションになる
  - 軌跡が残ったままになる
  - アプリがフリーズする
- 今回は表示維持より安定性を優先し、overlay を一旦無効化
- `lib.rs` に `ACTION_LABEL_OVERLAY_ENABLED = false` を追加
- 起動時 overlay 初期化停止、show / clear を no-op 化
- 少なくとも overlay が通常ジェスチャー経路へ介入しない状態に戻した

### 一覧の位置微調整

- `+` ボタンを追加行全体の中央へ配置
- `内容` 列を見出しの真下へ揃えやすいよう中央配置へ調整
- 4 列構成自体は維持

### アクション一覧のグループ化

- `Action` に `group` 項目を追加
- frontend / backend 双方で、未設定 group を `未分類` とする fallback を追加
- `ActionEditor` に `グループ名` 入力欄を追加
- `ActionList` を `group` ごとにまとめる構成へ変更
- 各グループに
  - 見出し
  - 件数
  - 折りたたみ / 展開
  を追加
- export / import は `config.actions` ごと保存しているため、`group` 情報も同梱される構成を維持
- 実ビルドは成功
- action タブの grouped 一覧スクリーンショットは、今回の自動切替では安定取得できず未確認
### 2026-06-28 追加修正: グループを独立単位へ再設計

- `Action.group` 文字列方式をやめ、`Config.groups + Action.group_id` 構造へ変更
- Rust 側 `config.rs` に migration を追加し、旧 `group` データを起動時に `groups/group_id` へ正規化
- `ActionEditor` からグループ名入力欄を削除し、所属グループは read-only 表示へ変更
- `ActionList` をグループ親 + アクション子の一覧へ更新
  - グループ見出し
  - 折りたたみ / 展開
  - 件数表示
  - グループ名インライン編集
  - グループ単位 `+` 追加
- 一覧上部に `+ グループを追加` を実装
- `useStore` の `config.actions` 同期を補強し、グループ編集時に `saveConfig` ベースで整合を保つよう修正
- `default-config.json` も `groups + group_id` 構造へ更新

### ビルド / ランタイム確認

- フロント build 成功
- Tauri release build 成功
- NSIS bundle 生成成功
  - `external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/GestureHotkeyApp.exe`
  - `external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/bundle/nsis/GestureHotkeyApp_0.1.0_x64-setup.exe`
- `%APPDATA%\\GestureHotkeyApp\\config.json` で `groups` 配列と `actions[].group_id` への migration を確認

### スクリーンショット

- grouped 一覧確認:
  - `docs/test-artifacts/grouped-actions-tauri-window.png`

### 未確認

- グループ名のインライン編集を最後まで実操作で確認したか
- グループ追加直後の rename UX
- アクションを別グループへ移動する UI

### 2026-07-03 追加: 別 PC Codex 用の引き継ぎ仕様書

- リポジトリ直下へファイルを集約
- `docs/codex-handoff-spec.md` を新規作成
- 新しい Codex が
  - 現在の主作業ディレクトリ
  - 実装済み範囲
  - 無効化中の機能
  - 次の優先順
  - ビルド手順
  を 1 本で把握できるよう整理
