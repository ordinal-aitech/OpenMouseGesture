# Windows インストーラー計画

## 目的

- `GestureHotkeyApp-setup.exe` 1 本で別 PC に導入できるようにする
- インストール後は `C:\Program Files\GestureHotkeyApp\` 配下へ入る通常の Windows アプリとして扱えるようにする

## 採用方針

- Tauri 2 の Windows bundler を使う
- インストーラー形式は `NSIS` を採用する
- 理由:
  - EXE 形式のセットアップを作れる
  - Tauri の公式設定で `installerIcon`、`uninstallerIcon`、`startMenuFolder`、`installMode` を扱える

## 名称方針

- 製品名: `GestureHotkeyApp`
- 実行ファイル名: `GestureHotkeyApp.exe`
- インストーラー成果物名: `GestureHotkeyApp-setup.exe`
- スタートメニュー表示名: `GestureHotkeyApp`
- Program Files 配下フォルダ名: `GestureHotkeyApp`

## インストール先方針

- 既定のインストールモードは `perMachine`
- 想定インストール先:
  - `C:\Program Files\GestureHotkeyApp\`

補足:

- Tauri 公式設定では NSIS の `installMode: "perMachine"` により、既定の配置先を `Program Files` 側へ寄せられる
- 実際の最終配置は NSIS 生成物で実ビルド確認が必要

## ショートカット方針

- スタートメニューに `GestureHotkeyApp` のショートカットを作成
- Start Menu folder も `GestureHotkeyApp` に統一
- デスクトップショートカットは標準挙動に従うが、最終挙動は実インストールで確認する
- ショートカットアイコンはアプリ本体と同一系統の `.ico` を使用する

## アンインストール方針

- NSIS 標準のアンインストール経路を使用
- アンインストーラー icon も `GestureHotkeyApp` 系アイコンを使う
- 将来的に設定ファイルを残すか削除するかは明示方針が必要
  - 初回は Tauri / アプリ既定挙動に従う
  - 追加要件が出たらアンインストーラー hook を検討する

## Tauri 設定反映方針

`src-tauri/tauri.conf.json` に以下を反映する。

```json
{
  "productName": "GestureHotkeyApp",
  "mainBinaryName": "GestureHotkeyApp",
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "windows": {
      "nsis": {
        "installMode": "perMachine",
        "startMenuFolder": "GestureHotkeyApp",
        "installerIcon": "icons/installer-icon.ico",
        "uninstallerIcon": "icons/uninstaller-icon.ico"
      }
    }
  }
}
```

## 生成手順

前提:

- Node.js
- Rust / Cargo
- Tauri CLI

想定コマンド:

```powershell
npm install
npm run tauri build
```

生成先の想定:

- `src-tauri/target/release/bundle/nsis/GestureHotkeyApp-setup.exe`

## 今回時点での未完了事項

- この実行環境には `node` と `cargo` が入っていないため、実ビルドと installer 生成は未確認
- 実成果物名、ショートカット作成結果、Program Files 配置結果は、ビルド環境整備後に再確認が必要
- 現在のソースでは `tauri.conf.json` へ NSIS 設定の反映まで実施済み

## 将来の追加候補

- コード署名
- WebView2 配布方式の明示
- インストーラー日本語化
- バナー画像やサイドバー画像の追加
- アップデーター連携

## 2026-06-28 確認結果

### 確認済み

- `npm run tauri build` により NSIS 生成まで完了
- 実生成物:
  - `src-tauri/target/release/bundle/nsis/GestureHotkeyApp_0.1.0_x64-setup.exe`
  - `src-tauri/target/release/bundle/nsis/GestureHotkeyApp-setup.exe`
- `GestureHotkeyApp.exe` の release ビルド生成を確認
- `installMode = perMachine`
- `startMenuFolder = GestureHotkeyApp`
- installer icon は `icons/installer-icon.ico` を参照

### 実装上の補足

- release 時の設定保存先は `Program Files` 直下ではなく `%APPDATA%\GestureHotkeyApp\` を正とする
- これにより per-machine install 後も通常権限ユーザーが設定変更できる
- `bundle.windows.nsis.uninstallerIcon` は現行 Tauri schema 非対応のため設定していない

### 未確認

- `GestureHotkeyApp-setup.exe` の実行による `C:\Program Files\GestureHotkeyApp\` 配置確認
- スタートメニューショートカット作成確認
- アンインストール導線確認
- インストール後アイコン表示確認
