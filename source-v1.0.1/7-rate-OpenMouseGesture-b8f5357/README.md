# OpenMouseGesture

[![CI](https://github.com/7-rate/OpenMouseGesture/actions/workflows/ci.yml/badge.svg)](https://github.com/7-rate/OpenMouseGesture/actions/workflows/ci.yml)

## プロジェクト概要

OpenMouseGestureは、Windowsで動作する高性能なマウスジェスチャーアプリケーションです。

### 特徴

- **シンプル・軽量・高速**: システムリソースの消費を最小限に抑えた設計
- **高度にカスタマイズ可能**: JSON形式でジェスチャーとアクションを自由に設定
- **豊富なアクションサポート**: キーストローク送信、アプリケーション起動、ウィンドウ操作、URLオープンなど
- **クリーンな配布**: OS環境やレジストリを汚さないZIP配布形式

## スクリーンショット
![ジェスチャー登録画面](docs/images/register_gesture.png)
![アクション登録画面](docs/images/actions.png)

## 機能一覧

### コア機能
- **マウスジェスチャー認識**: グローバル低レベルマウスフックで軌跡を収集しパターンマッチング
- **ホイール操作対応**: ホイールアップ/ダウン/クリック、左クリック+ホイール、X1/X2ボタンをトリガーとして使用可能
- **軌跡の視覚化**: ジェスチャーの軌跡をリアルタイムで画面表示（有効/無効切替可能）
- **ウィンドウ制御**: 対象ウィンドウや無視するウィンドウ（EXE単位）の管理

### アクションタイプ
- **キーストローク送信**: 任意のキーコンビネーション、メディアコントロール（音量調整、再生/一時停止）
- **コマンド実行**: 外部アプリケーションの起動、引数付きコマンド実行
- **URL オープン**: デフォルトブラウザでURLを開く
- **ウィンドウ操作**: ウィンドウの最小化、最大化、閉じる、etc.

## インストール手順
1. Releasesから最新のZIPファイルをダウンロード
2. 任意のフォルダに解凍
3. `OpenMouseGesture.exe`を実行


## スタートアップ登録方法

Windowsの起動時に自動実行するには：

1. **Win+R**を押して「ファイル名を指定して実行」を開く
2. `shell:startup`と入力してEnter
3. スタートアップフォルダが開くので、`OpenMouseGesture.exe`のショートカットを配置


## 開発者向け情報

### 技術スタック

- **バックエンド**: Rust + windows-rs
  - windows-rsでWindows APIを直接呼び出し
  - 低レベルマウスフックによるジェスチャー検出
- **フロントエンド**: Tauri + React + TypeScript
  - 宣言的UIによる設定画面
  - リアルタイムな設定反映

### 開発環境構築

#### 必要要件
- **Windows 10/11**
- **Rust**: 最新の安定版（`rustup`経由でインストール推奨）
- **Node.js**: LTS版（v18以上推奨）
- **Visual Studio Build Tools**: Windows開発ツール

#### セットアップ手順

1. **Visual Studio Build Toolsをインストール**
   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)からダウンロード
   - C++によるデスクトップ開発をインストール

2. **Node.js/Rustをインストール**
   ```powershell
   winget install -e --id OpenJS.NodeJS.LTS
   winget install rustlang.rustup
   ```

3. **リポジトリをクローン**
   ```git
   git clone https://github.com/7-rate/OpenMouseGesture.git
   ```

4. **依存関係をインストール**
   ```bash
   npm install
   ```

5. **cargo-aboutをインストール** (ライセンス情報生成用)
   ```bash
   cargo install cargo-about
   ```

### ビルド手順

#### デバッグ用開発ビルド
```bash
npx tauri dev
```
開発サーバーが起動し、ホットリロードが有効になります。
設定ファイルは`{プロジェクトルート}/config/`配下に配置されます。

#### リリースビルド
```bash
npx tauri build
```
生成物は`src-tauri/target/release/`配下に出力されます。

### 設定ファイル(config.json/gestures.json)の構造

設定ファイルの詳細は[docs/config.json.yaml](docs/config.json.yaml)を参照してください。  
ジェスチャーパターンの詳細は[docs/gestures.json.yaml](docs/gestures.json.yaml)を参照してください。  
通常、新しいジェスチャーはUI上で描画してから自動的に正規化されたデータが生成されます。

### コントリビューションガイドライン

1. **コーディング規約**
   - すべてのモジュールに日本語のヘッダーコメントを含める
   - snake_case命名規則を厳守

2. **プルリクエスト**
   - 変更内容を明確に説明すること

3. **Issue報告**
   - バグ報告には再現手順を含めること
   - 機能リクエストには用途と期待される動作を明記


## ライセンス

MIT License - 詳細は[LICENSE](LICENSE)を参照

依存ライブラリのライセンス情報は、アプリケーション内の「ライセンス」タブで確認できます。

---

Copyright (c) 2026 7-rate
