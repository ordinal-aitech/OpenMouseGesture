# アクション一覧 UI 調整

## 目的
- action 一覧で必要情報を過不足なく見せる
- trigger 情報を 1 箇所だけに整理する
- カードをややコンパクトにして一覧性を上げる

## 修正前の課題
- `Trigger B` などの trigger 表示が一覧内で重複していた
- Trigger 表示と gesture プレビューが縦に大きく、一覧が間延びして見えた
- 実行内容欄にも trigger 情報が入り、情報設計がやや散っていた

## 2026-06-28 再調整

### 修正内容
- `ActionList.tsx` / `ActionList.css` を再調整
- 一覧を以下の 4 列へ分離
  - アクション名
  - トリガー
  - ジェスチャー
  - 内容
- Trigger 列には `Trigger A / B / C` のみ表示
- ジェスチャー列にはプレビューとジェスチャー名のみ表示
- 内容列には `Ctrl+C` / `Ctrl+V` などの実行内容のみ表示
- 列見出しも 4 列に揃えた
- 右側 editor panel 幅を少し絞り、一覧列が収まりやすいようにした

### 今回の微調整
- `+` ボタンを、追加行全体の中央に来るよう調整
- 内容列は `action-col-action` を中央揃えにし、`action-desc` に `width: 100%` を持たせて、見出し `内容` の真下に入りやすいようにした
- 見出しと本文の整列方針:
  - アクション名: 左寄せ
  - トリガー: 中央寄せ
  - ジェスチャー: 中央寄せ
  - 内容: 中央寄せ

## `paste` 修正
- 保存済み設定 `%APPDATA%\GestureHotkeyApp\config.json` に `past` が残っていた
- `config.rs` の normalize で `past -> paste` へ補正するようにした
- 起動後、保存ファイルが `paste` に更新されることを確認した

## 確認結果
- 自動確認:
  - release ビルド成功
  - アプリ起動成功
  - 4 列一覧 UI のスクリーンショット取得済み
- スクリーンショット:
  - [action-list-ui-4col-after.png](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/docs/test-artifacts/action-list-ui-4col-after.png)

## 未確認事項
- マウス操作しながらの長時間一覧利用で押しやすさが十分か
- action 件数が多い場合の視認性
- `+` ボタン中央寄せと内容列位置が、ユーザー実機で十分自然に見えるか

## 関連する今回の追加
- 一覧の縦伸び対策として、別途 `group` によるグループ化と折りたたみを追加
- 詳細は [action-list-grouping-plan.md](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/docs/action-list-grouping-plan.md)
