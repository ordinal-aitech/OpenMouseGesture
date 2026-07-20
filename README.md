# OpenMouseGesture Workspace

このリポジトリには、OpenMouseGestureの現行ソース、仕様書、設計・検証記録、配布用ビルド手順を集約している。

## 現在の正本

- プロジェクト仕様書: `PROJECT.md`
- 変更履歴: `CHANGELOG.md`
- 現行ソース: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
- 設計・検証・過去の作業記録: `docs/`
- 配布物の受け渡し先: `dist/windows/`

`PROJECT.md` は現在の実装仕様をまとめた正本である。`docs/` 内の旧計画・進捗記録と内容が異なる場合は、現行コード、`PROJECT.md`、`CHANGELOG.md` を優先する。

## 現在の状態

2026年7月20日時点で、以下を実装・確認済みである。

- Tauri 2 + React + TypeScript + RustによるWindowsアプリ
- トレイ常駐とグローバルマウス／キーボードフック
- Trigger A / B / Cによるジェスチャー実行
- 左クリックをジェスチャー開始トリガーとして拒否する多層安全対策
- 通常の右クリックを維持する短クリック・パススルー
- 軌跡描画の安定化
- カスタムジェスチャー／アクション設定の保存
- デフォルト上書き前の設定バックアップ
- 21件のカスタムアクション復元
- 最新インストーラーによる上書き導入
- 実機での基本ジェスチャー動作確認
- 修飾キー付きキーボードトリガー（`Shift+F1`等）の信頼性向上（`GetAsyncKeyState`によるライブ状態の突き合わせ）
- Trigger A/B/C + ホイール方向単位でのホイールアクション解決、および左クリック依存の旧モデル (`leftclick_wheel_*`) の廃止・移行
- 修飾キー付きキーボードトリガー保持中でもホイールアクションが確実に発火するよう、送出直前だけ物理修飾キーを一時解除・送出後に復元する分離処理を追加
- ウィンドウ操作「最大化」を、最大化されていなければ最大化・最大化済みなら元のサイズへ復元するトグル動作に変更
- `command`（外部プログラム起動）とは別の `text` アクションを追加。クリップボードを使わず `SendInput` + `KEYEVENTF_UNICODE` でキャレット位置へ日本語・記号・改行を含む任意のテキストを直接入力
- アクション編集画面の「グループ」欄を読み取り専用表示から選択式に変更。既存アクションのグループを削除・再作成せずに他グループへ移動可能

当面は通常利用しながら継続テストする段階であり、現在判明している必須修正はない。

既存設定由来で、表示名と実際のアクション割り当てが一致していない項目が残る可能性はある。例として「全画面化」という名称が内部では最小化に割り当てられている設定があるため、実利用で問題になった場合は設定内容を個別に修正する。

## 主な配置

- `source-v1.0.1/`
  - OpenMouseGesture改造元ソース
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
  - 以前の .NET 8 / WPF試作版
  - 参照用であり、現行実装ではない

## 配布物 (`dist/windows/`)

最新のインストーラーと実行ファイルは、深いTauriビルド出力パスを直接たどらず、リポジトリ直下の `dist/windows/` から取得する。

生成手順:

1. `cd source-v1.0.1/7-rate-OpenMouseGesture-b8f5357 && npm run tauri build`
2. リポジトリ直下へ戻り `npm run dist:windows`

`dist/windows/` に生成されるもの:

- `OpenMouseGesture-x64.exe` — リリース実行ファイル
- `OpenMouseGesture-Setup-x64.exe` — NSISインストーラー
- `SHA256SUMS.txt` — 配布物のSHA-256
- `build-info.json` — バージョン、ビルド日時、commit SHA、各成果物のハッシュ

`dist/windows/` の `.exe` と生成メタデータはGit管理対象外である。詳細は `dist/README.md` を参照する。

## 仕様・検証を確認する順序

1. `PROJECT.md`
2. `CHANGELOG.md`
3. 現行ソースとテスト
4. `docs/test-result-summary.md`
5. `docs/progress-log.md`
6. `docs/openmousegesture-mod-plan.md` や `docs/codex-handoff-spec.md` などの過去資料

過去資料は経緯確認には使えるが、現行仕様の正本として扱わない。