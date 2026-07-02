# テスト結果サマリー

## ChatGPT へ渡す要約
- 2026-06-28 時点で `GestureHotkeyApp.exe` の release ビルドと NSIS 生成は成功している
- アクション一覧 UI を 4 列構成へ整理した
- `Action.group` を追加し、グループ見出しと折りたたみ UI を実装した
- 保存済み設定に残っていた `past` は、起動時 normalize で `paste` へ補正されるように修正し、実ファイル反映も確認した
- アクションラベルオーバーレイは、ユーザー実機でカーソル低速化・軌跡残り・フリーズを引き起こしたため、今回いったん無効化した
- 現在の最優先は、通常のジェスチャー動作を壊さない状態へ戻すこと
- 本 md では、自動確認と物理マウス確認を分けて記載する

## テスト実施日
- 2026-06-28

## テスト環境
- OS: Windows
- アプリ: `GestureHotkeyApp` release build
- ビルド方式: `npm run tauri build`

## 対象ファイル
- [ActionList.tsx](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src/components/actions/ActionList.tsx)
- [ActionList.css](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src/components/actions/ActionList.css)
- [mouse_hook.rs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/mouse_hook.rs)
- [config.rs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/config.rs)
- [action_label_overlay.rs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/external/OpenMouseGesture/source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/action_label_overlay.rs)

## 自動確認で成功した項目

| 項目 | 結果 | 補足 |
|---|---|---|
| release ビルド | 成功 | `GestureHotkeyApp.exe` 生成確認 |
| NSIS 生成 | 成功 | `GestureHotkeyApp_0.1.0_x64-setup.exe` 生成確認 |
| アプリ起動 | 成功 | `GestureHotkeyApp` プロセス起動確認 |
| アクション一覧 4 列化 | 成功 | スクリーンショットで確認 |
| Trigger / ジェスチャー列の分離 | 成功 | スクリーンショットで確認 |
| Action.group 追加 | 成功 | frontend / backend 双方へ反映 |
| 未設定 group の `未分類` fallback | 成功 | `Action::normalized()` で補正 |
| グループ header / 折りたたみ UI | 成功 | `ActionList.tsx` 実装済み |
| グループ情報の export / import 同梱 | 成功 | `config.actions` に含まれる構成 |
| `past` -> `paste` 補正 | 成功 | `%APPDATA%\\GestureHotkeyApp\\config.json` 更新確認 |
| action label overlay 無効化 | 成功 | `ACTION_LABEL_OVERLAY_ENABLED = false` で no-op 化 |
| overlay 無効化後ビルド | 成功 | release build 成功 |

## 物理マウス実機で未確認の項目

| 項目 | 状態 | 理由 |
|---|---|---|
| overlay 無効化後に重さが解消したか | 未確認 | 物理マウス再確認は未実施 |
| overlay 無効化後に軌跡残りが解消したか | 未確認 | 物理マウス再確認は未実施 |
| overlay 無効化後にフリーズが解消したか | 未確認 | 物理マウス再確認は未実施 |
| `+` ボタン中央寄せの見え方 | 未確認 | ユーザー実機確認待ち |
| 内容列の位置調整の見え方 | 未確認 | ユーザー実機確認待ち |
| グループ化 UI の実画面確認 | 未確認 | action タブ自動切替の画面取得が今回不安定 |
| 折りたたみ動作の実機確認 | 未確認 | 物理クリック確認は未実施 |

## スクリーンショット確認
- 一覧 UI 修正後:
  - [action-list-ui-4col-after.png](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/docs/test-artifacts/action-list-ui-4col-after.png)

## 問題が出た条件
- 既存保存設定 `%APPDATA%\\GestureHotkeyApp\\config.json` に `name = "past"` が残っていた
- ラベルオーバーレイは、ユーザー実機で以下の重大不具合が確認された
  - マウスカーソルがスローモーションになる
  - 軌跡が残ったままになる
  - マウスジェスチャーソフト自体がフリーズする

## 暫定対応の有無
- あり
  - `Action::normalized()` に `past -> paste` 補正を追加
  - action label overlay は一旦無効化
  - 起動時 overlay 初期化を停止
  - show / clear を no-op 化
  - 一覧 4 列構成は維持したまま、`+` ボタン中央寄せと内容列位置調整を追加
  - `group` 項目追加と `未分類` fallback を追加
# 2026-06-28 追加確認: グループ独立化

## 自動確認で成功
- `npm run build` 成功
- `npm run tauri build` 成功
- `GestureHotkeyApp.exe` 生成成功
- `GestureHotkeyApp_0.1.0_x64-setup.exe` 生成成功
- 旧 `Action.group` 文字列データを含む `%APPDATA%\\GestureHotkeyApp\\config.json` を、起動後に `groups + group_id` 構造へ migration できた
- grouped 一覧 UI の表示をスクリーンショットで確認できた
  - `docs/test-artifacts/grouped-actions-tauri-window.png`

## 実装済み
- `Config.groups` を追加
- `Action.group_id` を追加
- `ActionEditor` からグループ名入力欄を削除
- グループ見出し単位の折りたたみ / 展開
- グループ見出しからの `+` 追加導線
- 一覧上部の `+ グループを追加`
- export / import 対象へ `groups` と `group_id` を含める構造に変更

## 未確認
- グループ名インライン編集の実操作
- 新規グループ追加後の rename 操作
- アクションを別グループへ移動する導線
- 物理マウスでの長時間利用時の一覧 UX

## 補足
- 今回は「グループ名を action ごとに入力する方式」をやめる修正が主目的
- action label overlay は前回方針どおり無効化のまま維持
