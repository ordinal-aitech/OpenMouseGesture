# アプリ名とアイコン方針

## 正式名称

- アプリ名: `GestureHotkeyApp`
- 製品名: `GestureHotkeyApp`
- 実行ファイル名: `GestureHotkeyApp.exe`
- インストーラー名: `GestureHotkeyApp-setup.exe`

## ブランド方針

- 自己使用前提なので過度なブランド演出は行わない
- ただし Windows アプリとして違和感のない見た目に揃える
- OpenMouseGesture 由来の名称は、ユーザーから見える範囲では `GestureHotkeyApp` に置き換える

## アイコン方針

### 目的

- マウスジェスチャー + Hotkey 用途が伝わること
- 小さいサイズでも潰れにくいこと
- タスクトレイ、スタートメニュー、エクスプローラー、インストーラーで見やすいこと

### デザイン方針

- ベースは濃い背景の角丸スクエア
- メインモチーフはオレンジ系のジェスチャー軌跡
- 補助モチーフとして Hotkey を想起させるシンプルなキー形状を重ねる
- 細かい文字は避け、32px でも判別できる単純形状を優先する

### 使い分け

- アプリ本体アイコン:
  - `32x32.png`
  - `128x128.png`
  - `128x128@2x.png`
  - `icon.ico`
- インストーラー用アイコン:
  - `installer-icon.ico`
  - `uninstaller-icon.ico`

補足:

- 初回は同一モチーフを流用してよい
- 将来的にアプリ本体とインストーラーで微差分を作る余地は残す

## UI 反映方針

- Tauri の app icon に反映
- Info タブのアイコン表示に反映
- スタートメニューとショートカットに反映
- NSIS installer / uninstaller に反映

## 名称反映方針

以下の表示を `GestureHotkeyApp` に揃える。

- `tauri.conf.json` の `productName`
- `tauri.conf.json` の window title
- `index.html` の title
- `InfoTab.tsx` の表示名
- `package.json` の package name

## 今回の実装優先順位

1. 名前の統一
2. app / installer icon の配置
3. Tauri 設定への反映
4. 実ビルド後の見え方確認

## 現状

- `GestureHotkeyApp` 名称は Tauri 設定、`package.json`、`index.html`、`InfoTab` へ反映済み
- `icon.ico`、`installer-icon.ico`、`uninstaller-icon.ico` は生成済み
- 実インストーラー画面と Start Menu 表示は未ビルドのため未確認

## 2026-06-28 確認結果

### 反映済み

- `productName`: `GestureHotkeyApp`
- `mainBinaryName`: `GestureHotkeyApp`
- 設定画面タイトル: `GestureHotkeyApp 設定`
- 生成 exe 名: `GestureHotkeyApp.exe`
- 配布用 installer 名: `GestureHotkeyApp-setup.exe`
- `src-tauri/icons` に app / installer / uninstaller 向け icon を配置済み

### 未確認

- 実インストール後のショートカット名とアイコン表示
- スタートメニューでの見え方
- 実行中トレイアイコンの見え方
