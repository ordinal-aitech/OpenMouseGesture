# GestureHotkeyApp ビルド環境メモ

## 現在不足している環境

この作業環境では以下が未導入でした。

- `Node.js`
- `cargo` / `rustc`
- `git`

そのため、以下は未実施です。

- `npm install`
- `npm run tauri build`
- NSIS インストーラー生成
- 実行ファイル起動確認

## 導入が必要なもの

最低限:

1. Node.js 20 以降
2. Rust toolchain
3. Visual Studio Build Tools または Visual Studio の C++ build tools
4. WebView2 Runtime
5. Git

推奨:

- `rustup`
- `pnpm` または `npm`
- `tauri` CLI は package.json 側の devDependency を使用

## 導入後に実行するコマンド

作業ディレクトリ:

`C:\Users\ohkat\OneDrive\ドキュメント\Windowsアプリ開発\external\OpenMouseGesture\source-v1.0.1\7-rate-OpenMouseGesture-b8f5357`

```powershell
node --version
cargo --version
git --version

npm install
npm run tauri build
```

## ビルド後の確認ポイント

### 1. 生成物確認

- `src-tauri/target/release/` に `GestureHotkeyApp.exe` が出ること
- `src-tauri/target/release/bundle/nsis/` に `GestureHotkeyApp-setup.exe` が出ること

### 2. アプリ起動確認

- `GestureHotkeyApp.exe` を起動
- トレイアイコンが表示されること
- SettingsTab に `トリガーボタン設定` が表示されること
- Trigger A / B / C の開始ボタンと軌跡色が編集できること

### 3. 設定保存確認

- Trigger A / B / C の開始ボタンを変更
- Trigger A / B / C の色を変更
- 再起動後も設定が保持されること

### 4. ジェスチャー確認

- Trigger A / B / C それぞれでジェスチャー開始できること
- 同じ gesture でも trigger slot ごとに別 Hotkey が実行されること
- 軌跡色が slot ごとの設定色になること
- 認識後に action label overlay が表示されること

### 5. インストーラー確認

- `GestureHotkeyApp-setup.exe` でインストールできること
- 既定インストール先が `C:\Program Files\GestureHotkeyApp\` になること
- スタートメニューに `GestureHotkeyApp` が作成されること
- アイコンが app / installer / shortcut に反映されること

## 注意

- 現在のソース変更は未ビルドのため、Rust / TypeScript の型崩れや Tauri command 名の不整合が残っている可能性があります
- 実ビルド時はまず TypeScript エラー、その後 Rust コンパイルエラーの順に潰す想定です
