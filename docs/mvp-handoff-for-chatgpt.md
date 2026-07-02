# MVP 引き継ぎメモ for ChatGPT

## ChatGPTへ渡す要約

- Windows 11 専用の常駐型マウスジェスチャーアプリ `GestureHotkeyApp` の MVP を `.NET 8 + WPF + Win32 P/Invoke` で実装済み。
- 対象機能は Hotkey 送信のみ。`RunProgram / URL起動 / Window操作 / Mail / Plugin / Lua` は未実装。
- Trigger A / Trigger B の 2 系統を持ち、開始ボタンは `Middle / Right / XButton1 / XButton2` から個別設定できる。
- 左ボタンは開始ボタン候補に含めていない。
- 日本語 UI、トレイ常駐、JSON 保存、グローバルアクション、アプリ別プロファイル UI を実装済み。
- `dotnet build` は成功。アプリ起動、タブ遷移、設定保存、JSON 反映は確認済み。
- `StartWithWindows` は `config.json` に保存されるが、`HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` に未反映だった。
- 実マウスジェスチャーから Hotkey が送信される end-to-end は未確認。
- 学習 UI、未認識ジェスチャー通知、描画線、テストコードは未実装。

---

## 1. アプリ概要

- アプリ名: `GestureHotkeyApp`
- 目的:
  - Windows 11 上でマウスジェスチャーを認識し、設定済み Hotkey を送信する
- 制約:
  - Windows 11 専用
  - UI は日本語
  - 左ボタンは開始ボタンとして使わない
  - Hotkey 送信以外の機能は MVP 対象外

## 2. 現在実装済みの機能

### 常駐・OS連携

- WPF デスクトップアプリ
- トレイアイコン表示
- 有効 / 無効切替
- アプリ終了時に通常は閉じず、非表示化して常駐継続
- 自動起動設定の UI と設定保存

### 入力・認識

- 低レベルマウスフック
- Trigger A / Trigger B の 2 系統
- 各 Trigger の開始ボタン設定
  - `Middle`
  - `Right`
  - `XButton1`
  - `XButton2`
- 8方向ベースのジェスチャー正規化
  - `R / L / U / D / UR / UL / DR / DL`
- 短い押し離し時の元クリック再生

### 実行

- `SendHotkey` のみ実装
- Hotkey 文字列例:
  - `Alt+Left`
  - `Ctrl+Shift+S`
  - `Win+1`
  - `F5`

### 設定・UI

- 日本語 UI
- グローバルアクション編集
- アプリ別プロファイル UI
- プロファイル条件:
  - `process`
  - `class`
  - `title`
- `include / exclude` モード
- 設定保存
- 設定再読み込み
- 設定インポート / エクスポート
- 最終認識ジェスチャー表示

### 保存

- `%AppData%\\GestureHotkeyApp\\config.json` に JSON 保存

## 3. 未実装機能

### Phase 2 以降

- 学習 UI
- 未認識ジェスチャー通知
- ジェスチャー描画線
- Trigger ごとの描画色分離
- テストコード

### 今回 intentionally 未対象

- RunProgram
- URL 起動
- Mail
- Window 操作
- 外部 DLL プラグイン
- Lua
- OSD
- Win32 message send/post
- マルチモニタ専用機能
- 左ボタン開始

## 4. 技術構成

- 言語: C#
- ランタイム: .NET 8
- UI: WPF
- OS 連携:
  - `SetWindowsHookEx` による低レベルマウスフック
  - `SendInput` による Hotkey 送信
  - `GetForegroundWindow / GetClassName / GetWindowText / GetWindowThreadProcessId`
  - `NotifyIcon`
  - レジストリ自動起動
- 保存方式: JSON

## 5. 主要ファイル一覧

### エントリと UI

- [App.xaml](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/App.xaml)
  - WPF アプリ定義
- [App.xaml.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/App.xaml.cs)
  - 起動時 DI 相当の組み立て
  - `ApplicationController` 初期化
  - トレイサービス接続
- [MainWindow.xaml](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/MainWindow.xaml)
  - 日本語設定 UI
- [MainWindow.xaml.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/MainWindow.xaml.cs)
  - 画面イベント
  - 保存 / 読み込み / Import / Export のトリガ

### ViewModel

- [MainWindowViewModel.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/ViewModels/MainWindowViewModel.cs)
  - 画面状態
  - Trigger 設定
  - グローバルアクション
  - プロファイル操作

### アプリ中核

- [ApplicationController.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/ApplicationController.cs)
  - 実行制御の中核
  - 設定ロード / 保存
  - ジェスチャー受信後の解決と Hotkey 実行

### 入力・認識・実行

- [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs)
  - マウスフック
  - Trigger A / B の押下開始判定
  - ポイント収集
- [GestureRecognitionService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/GestureRecognitionService.cs)
  - 方向列への変換
- [HotkeySenderService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/HotkeySenderService.cs)
  - Hotkey 文字列の解釈
  - `SendInput` 実行
- [ProfileResolverService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/ProfileResolverService.cs)
  - include / exclude 判定
- [WindowInfoService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/WindowInfoService.cs)
  - 前景ウィンドウ情報取得

### 保存・トレイ・OS設定

- [JsonConfigurationService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/JsonConfigurationService.cs)
  - `config.json` のロード / 保存 / Import / Export
- [StartupRegistrationService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/StartupRegistrationService.cs)
  - 自動起動レジストリ設定
- [TrayIconService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/TrayIconService.cs)
  - トレイメニュー

### モデル

- [AppConfiguration.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/AppConfiguration.cs)
- [AppProfile.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/AppProfile.cs)
- [GestureAction.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/GestureAction.cs)
- [TriggerSettings.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/TriggerSettings.cs)
- [ProfileMatcher.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/ProfileMatcher.cs)
- [HotkeyDefinition.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/HotkeyDefinition.cs)
- [Enums.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Models/Enums.cs)

### Win32 宣言

- [NativeMethods.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Native/NativeMethods.cs)

## 6. 現在の課題

### 事実として確認できた課題

- `Windows 起動時に開始する` を ON にして保存しても、
  - `%AppData%\\GestureHotkeyApp\\config.json` には `StartWithWindows: true` が保存される
  - しかし `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\\GestureHotkeyApp` は作成されなかった
- 実マウスジェスチャーから Hotkey 送信までの end-to-end は未確認

### 実装上の制約

- 学習 UI がないため、認識結果を見ながら手動でジェスチャー文字列を入れる前提
- ジェスチャー描画がないため、入力中の視覚フィードバックがない
- Hotkey 入力欄が自由入力テキストで、専用エディタではない

## 7. 次にやるべき改修候補

### 優先度高

- 自動起動レジストリ未反映の修正
- ジェスチャー認識 → Hotkey 送信の end-to-end 動作確認
- Trigger A / B の重複禁止と反映の再確認

### 優先度中

- 学習 UI の追加
- 未認識ジェスチャー時の導線
- Hotkey 入力欄の専用入力 UI 化
- プロファイル編集 UX 改善

### 優先度低

- 描画線表示
- 認識結果プレビュー改善
- テストコード追加

## 8. ChatGPT に相談したい論点

- `StartWithWindows` が JSON には保存されるのに Run キーへ反映されない原因切り分け
- ジェスチャー認識の MVP として、現行の 8 方向文字列で十分か
- Hotkey 入力 UI を自由入力のままにするか、専用キーバインド入力コンポーネントにするか
- Trigger A / Trigger B とアプリ別プロファイルのデータモデルをこのまま維持するか
- 学習 UI を入れる前に、最低限どこまで自動テストを足すべきか
