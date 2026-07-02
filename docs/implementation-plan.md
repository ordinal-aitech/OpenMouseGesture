# 実装計画

作成日: 2026-06-23

## 採用技術

- アプリ種別: Windows 10 / 11 対応デスクトップ常駐アプリ
- 言語 / フレームワーク: C# / .NET 8 / WPF
- 常駐・OS連携:
  - Win32 低レベルマウスフック
  - `SendInput`
  - レジストリによる自動起動
  - `NotifyIcon` によるトレイ
- 保存方式: JSON

## この構成を採った理由

- Windows 10 / 11 で共通利用できる WPF + Win32 構成として素直
- 追加ランタイムなしで常駐・トレイ・入力フックを組みやすい
- Hotkey 送信を P/Invoke で閉じられる
- UI を日本語で早く組める

## ディレクトリ構成案

```text
src/GestureHotkeyApp
├─ Models
│  ├─ AppConfiguration
│  ├─ AppProfile
│  ├─ GestureAction
│  ├─ ProfileMatcher
│  ├─ TriggerSettings
│  └─ HotkeyDefinition
├─ Native
│  └─ NativeMethods
├─ Services
│  ├─ ApplicationController
│  ├─ MouseHookService
│  ├─ GestureTrailOverlayService
│  ├─ GestureRecognitionService
│  ├─ ProfileResolverService
│  ├─ HotkeySenderService
│  ├─ JsonConfigurationService
│  ├─ StartupRegistrationService
│  ├─ TrayIconService
│  └─ WindowInfoService
├─ ViewModels
│  └─ MainWindowViewModel
├─ App.xaml
├─ MainWindow.xaml
└─ GestureHotkeyApp.csproj
```

## Phase 1 の実装計画

### 1. アプリ土台

- WPF アプリ作成
- トレイ常駐
- アプリ終了制御

### 2. 設定保存

- JSON スキーマ作成
- `%AppData%` 保存
- 保存 / 再読み込み

### 3. Trigger A / Trigger B

- ボタン候補
  - 中ボタン
  - 右ボタン
  - XButton1
  - XButton2
- 重複禁止
- ランタイム更新

### 4. 入力取得

- 低レベルマウスフック
- ジェスチャー用ポイント列の収集
- 短いクリックの再生

### 5. 認識

- 8方向正規化
- `R/L/U/D/UR/UL/DR/DL`
- 方向列へ圧縮

### 6. 実行

- SendHotkey のみ
- 前景ウィンドウを維持したまま送信

### 7. プロファイル

- グローバルプロファイル
- アプリ別 include / exclude
- process / class / title 判定

### 8. 日本語 UI

- 基本設定
- グローバルアクション編集
- アプリ別プロファイル編集
- Import / Export

## Phase 2 で入れるもの

- 学習 UI
- 未認識ジェスチャーの導線
- 設定 UI の使い勝手改善
- ワイルドカード UI の磨き込み

## MVP 完了条件

- トレイ常駐で常時動作する
- Trigger A / B で別ボタンを設定できる
- ジェスチャーから Hotkey が送信される
- グローバルとアプリ別で割り当てを分けられる
- 設定の保存 / 復元 / 移行ができる
