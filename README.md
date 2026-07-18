# OpenMouseGesture Workspace

このリポジトリ配下に、今回の作業で使っていたファイル一式を集約しました。

## 主な配置

- `source-v1.0.1/`
  - OpenMouseGesture 改造元ソース
  - 実装変更の主対象
- `release-v1.0.1/`
  - 元の配布物
- `docs/`
  - 要件整理
  - 改修設計
  - 進捗ログ
  - テスト結果
  - スクリーンショット
- `artifacts/`
  - ビルド補助スクリプト
  - アイコン確認用画像
  - 旧試作の出力物
- `legacy/GestureHotkeyApp-Wpf/`
  - 以前の .NET 8 / WPF 試作版
  - `GestureHotkeyApp.sln`
  - `src/`
  - 当時の README

## いまの基準

- 現在の改修対象は `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
- 設計書と検証記録は `docs/`
- 旧 WPF 試作は参照用として `legacy/` に隔離

## 配布物 (dist/windows/)

最新のインストーラーと実行ファイルは、深い Tauri のビルド出力パス
(`source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/target/release/...`)
を直接たどらなくても、リポジトリ直下の `dist/windows/` から取得できます。

1. `cd source-v1.0.1/7-rate-OpenMouseGesture-b8f5357 && npm run tauri build` でリリースビルドを作成する
2. リポジトリ直下に戻り `npm run dist:windows` を実行する

`dist/windows/` に生成されるもの:

- `OpenMouseGesture-x64.exe` — リリースビルドの実行ファイル
- `OpenMouseGesture-Setup-x64.exe` — NSIS インストーラー
- `SHA256SUMS.txt` — 上記2ファイルの SHA-256
- `build-info.json` — バージョン、ビルド日時、コミットSHA、各成果物のハッシュ

`dist/windows/` の中身（`.exe` とメタデータ）は Git 管理対象外です。詳細は `dist/README.md` を参照してください。

## 補足

- `docs/progress-log.md`
- `docs/test-result-summary.md`
- `docs/openmousegesture-mod-plan.md`
- `docs/codex-handoff-spec.md`

あたりを開くと、ここまでの流れを追いやすいです。
