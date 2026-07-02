# アクション一覧のグループ化設計メモ

## 方針
- `Action.group` 文字列を各アクションで直接編集する方式は廃止する
- グループを独立したデータ単位として扱う
- アクションは `group_id` でグループを参照する
- 一覧 UI は `グループ親 + アクション子` の構造にする
- 既存の 4 列構成
  - アクション名
  - トリガー
  - ジェスチャー
  - 内容
  をグループ配下で維持する

## 新しいデータモデル

### Config
- `groups: ActionGroup[]` を追加

### ActionGroup
- `id: string`
- `name: string`

### Action
- `group_id?: string`
- 旧 `group` は migration 用の legacy 入力としてのみ扱い、保存時には出力しない

## action との参照関係
- 各アクションは `group_id` で所属グループを持つ
- グループ名変更は `groups` 側だけ更新する
- 同一グループ配下の全アクションに、一覧表示上は自動で反映される

## migration / fallback 方針
- 旧データで `group` 文字列しか無い場合:
  - 読み込み時に `groups` を生成
  - 同名 `group` は 1 つの `ActionGroup` に集約
  - 各アクションへ `group_id` を割り当てる
- `group` 未設定の旧データは `未分類` に割り当てる
- `groups` が空でも、`group-uncategorized / 未分類` を補完する
- 正規化後は `config.json` を新構造で再保存する

## UI 上のグループ編集方法
- `ActionList` にグループ見出し行を追加
- 見出し行は以下を持つ
  - 折りたたみ / 展開トグル
  - グループ名
  - 件数
  - そのグループ配下へ追加する `+`
- グループ名は見出し側でインライン編集する
- `ActionEditor` からはグループ名入力欄を外す
- `ActionEditor` では所属グループを read-only 表示に留める

## グループ配下追加方法
- グループ行の `+` から、そのグループ配下へ新規アクションを追加する
- 追加時に `group_id` を初期値として埋める
- 全体用の「アクション追加」導線ではなく、グループ単位追加を優先する
- グループ自体の新規作成は一覧上部の `+ グループを追加` から行う

## export / import との関係
- `groups` は `config` の一部として export / import 対象に含める
- `actions[].group_id` もそのまま settings bundle に含める
- 別 PC で import した場合も、グループ構造ごと復元できる前提にする

## 実装済みファイル
- `src/types/index.ts`
- `src/store/useStore.ts`
- `src/components/actions/ActionsTab.tsx`
- `src/components/actions/ActionList.tsx`
- `src/components/actions/ActionEditor.tsx`
- `src/components/actions/ActionList.css`
- `src/components/actions/ActionEditor.css`
- `src-tauri/src/config.rs`
- `config/default-config.json`

## 今後の拡張余地
- グループ削除
- グループ並び替え
- アクションの別グループ移動 UI
- 折りたたみ状態の永続化
- グループごとの色やアイコン設定

## 今回確認できたこと
- `config.json` の旧 `group` ベースデータは、起動後に `groups + group_id` 構造へ移行された
- 一覧 UI はグループ見出し + 折りたたみ構造に更新された
- スクリーンショット:
  - `docs/test-artifacts/grouped-actions-tauri-window.png`

## 未確認事項
- グループ名のインライン編集操作を実機で最後まで確認したか
- グループ追加後に即編集するフローの UX 調整
- アクションを別グループへ移動する導線
