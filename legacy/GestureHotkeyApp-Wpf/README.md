# GestureHotkeyApp

Windows 10 / 11 対応の常駐型マウスジェスチャーアプリです。  
用途は「ジェスチャーを認識して Hotkey を送信すること」に限定しています。

## 採用技術

- .NET 8
- WPF
- Win32 P/Invoke
  - 低レベルマウスフック
  - `SendInput`
  - 前景ウィンドウ情報取得
- JSON 設定保存
- `NotifyIcon` によるトレイ常駐

## いま入っている範囲

- バックグラウンド常駐
- トレイアイコン
- 有効 / 無効切替
- Windows 起動時の自動起動
- Trigger A / Trigger B
- 開始ボタンの個別設定
  - 中ボタン
  - 右ボタン
  - XButton1
  - XButton2
- ジェスチャー認識
- ジェスチャー軌跡のリアルタイム表示
  - Trigger A: 赤
  - Trigger B: 青
- Hotkey 送信
- グローバルアクション
- アプリ別プロファイル
- 設定保存 / 再読み込み
- 設定のインポート / エクスポート
- 日本語 UI

## 今回 intentionally 入れていないもの

- RunProgram
- URL 起動
- Window 操作
- Mail
- プラグイン
- Lua
- 左ボタン開始

## ディレクトリ構成

```text
.
├─ src/
│  └─ GestureHotkeyApp/
│     ├─ Models/
│     ├─ Native/
│     ├─ Services/
│     ├─ ViewModels/
│     ├─ App.xaml
│     ├─ MainWindow.xaml
│     └─ GestureHotkeyApp.csproj
├─ docs/
│  ├─ change-summary.md
│  ├─ implementation-plan.md
│  ├─ progress-log.md
│  ├─ requirements-draft.md
│  ├─ requirements-draft-v2.md
│  └─ software-analysis.md
└─ GestureHotkeyApp.sln
```

## ビルド

```powershell
dotnet build GestureHotkeyApp.sln
```

## 実行

```powershell
dotnet run --project src\GestureHotkeyApp\GestureHotkeyApp.csproj
```

## 設定ファイル

既定では以下に保存します。

```text
%AppData%\GestureHotkeyApp\config.json
```

## ジェスチャー記法

- `R` 右
- `L` 左
- `U` 上
- `D` 下
- `UR` 右上
- `UL` 左上
- `DR` 右下
- `DL` 左下

複数方向は `-` で連結します。

例:

- `L`
- `R`
- `U-D`
- `DR-UL`

## Hotkey 記法

例:

- `Ctrl+Shift+S`
- `Alt+Left`
- `Win+1`
- `F5`

## 注意

- MVP のため、学習 UI はまだ入れていません
- 代わりに最終認識ジェスチャーを画面に表示します
- 右クリックや中クリックを通常クリックとして使いたい場合、短い押し離しは元のクリックを再生します
