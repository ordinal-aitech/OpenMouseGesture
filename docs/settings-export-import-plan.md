# 設定 export / import 計画

## 採用するファイル形式

- 初回は JSON 形式
- 1 ファイルで設定一式を扱う
- 既定ファイル名:
  - `GestureHotkeyApp-settings.gha.json`

## export 対象項目

- `Config` 全体
  - `trajectory`
  - `ignore_exe`
  - `triggerA / triggerB / triggerC`
  - `triggerAColor / triggerBColor / triggerCColor`
  - `actions`
- gesture 一覧
- 付加情報
  - `formatVersion`
  - `appName`
  - `exportedAt`

## import 対象項目

- export bundle に含まれる `config`
- export bundle に含まれる `gestures`

## 保存 / 読込フロー

### export

1. SettingsTab の `設定をエクスポート` を押す
2. 保存ダイアログを開く
3. 現在の `config + gestures` を読み込む
4. bundle JSON を指定パスへ保存する

### import

1. SettingsTab の `設定をインポート` を押す
2. 現在設定を上書きする確認を出す
3. ファイル選択ダイアログで JSON を選ぶ
4. bundle を validation / normalize する
5. `gestures.json` と `config.json` へ保存する
6. フロントエンド側で再読込し、即時反映する

## 不正ファイル時の扱い

- JSON パース失敗:
  - import を中止
- `formatVersion` が不正:
  - import を中止
- gesture validation 失敗:
  - import を中止
- config validation 失敗:
  - import を中止

## migration との関係

- import 時も `Config.normalized()` を通す
- 旧形式で `trigger_slot` や `triggerA/B/C` が無い場合でも fallback で補完する

## UI 追加箇所

- `SettingsTab`
- 追加導線:
  - `設定をエクスポート`
  - `設定をインポート`

## 今後の注意点

- `exportedAt` は将来 ISO8601 にしたい
- より厳密なアトミック import が必要なら、テンポラリ保存と入れ替え方式へ拡張する

## 2026-06-28 実装反映メモ

- `SettingsBundle` を追加し、`formatVersion` / `appName` / `exportedAt` / `config` / `gestures` を 1 つの JSON にまとめる構成にした
- `exportedAt` は現状では Unix time 秒文字列
- 既定ファイル名は `GestureHotkeyApp-settings.gha.json`
- `SettingsTab` から save dialog / open dialog を使って export / import する
- import 前に上書き確認を出す
- import 後は `Config` と `gestures` を再読込して画面へ即時反映する
- release ビルドの実保存先は `%APPDATA%\GestureHotkeyApp\`
- 旧 release 配置の `config.json` / `gestures.json` は、新保存先へ初回移行する fallback を追加

## 未確認項目

- export した bundle を import して Trigger 設定、色設定、gesture 一覧、action 一覧が同一状態へ戻るか
- 別PC相当の復元フローで問題なく再現できるか
