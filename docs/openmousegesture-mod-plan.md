# OpenMouseGesture 改造計画

## 目的

- OpenMouseGesture をベースに、`GestureHotkeyApp` として自己使用向けの常駐型マウスジェスチャーアプリへ改造する
- 既存 UI はできるだけ流用し、全面作り直しは行わない
- ジェスチャーごとの機能ではなく、`trigger_slot + gesture` ごとの Hotkey 割り当てを中核に再設計する

## 正式名称方針

- 製品名: `GestureHotkeyApp`
- 実行ファイル名: `GestureHotkeyApp.exe`
- インストーラー名: `GestureHotkeyApp-setup.exe`
- Program Files 配下フォルダ名: `GestureHotkeyApp`
- スタートメニューのショートカット表示名: `GestureHotkeyApp`

補足:

- Tauri の `productName` と `mainBinaryName` を `GestureHotkeyApp` に揃える
- Windows Installer は NSIS ベースで EXE 化する
- 内部 crate 名や一部 OSS 由来のコード識別子は、初回段階では最低限の変更に留める

## 現状構造の要約

### フロントエンド

- `React + TypeScript + Zustand`
- 主要タブは `Gestures / Actions / Settings / Licenses / Info`
- 設定画面は `SettingsTab.tsx` と `SettingsTab.css` が担当

### バックエンド

- `Tauri 2 + Rust + windows-rs`
- グローバルマウスフック、ジェスチャー認識、設定保存、軌跡オーバーレイ描画を Rust 側で処理

### 現状の制約

- ジェスチャー開始ボタンは実質 Right 前提
- アクションは `gesture` 単位で結び付いており、同一ジェスチャーを trigger 別に分けられない
- 軌跡色は trigger slot ごとの設定参照になっていない

## 現在の入力処理で確認できたこと

### UI 上で既に見えている候補

- wheel 系アクション候補として以下が既に UI に存在する
  - `wheel_up`
  - `wheel_down`
  - `wheel_click`
  - `x1_button`
  - `x2_button`
  - `leftclick_wheel_up`
  - `leftclick_wheel_down`

### 実装上の確認

- `mouse_hook.rs` では `WM_RBUTTONDOWN` を起点としたジェスチャー開始処理が中心
- `WM_XBUTTONDOWN` を使う余地はあり、`XBUTTON1 / XBUTTON2` を trigger 候補へ拡張する実装は可能
- `Middle` も同様に候補化できる見込み
- `Left` は既存要件どおり trigger 候補へ含めない

## データモデル変更案

### Trigger まわりの型

```ts
export type GestureTriggerButton = "right" | "middle" | "x1" | "x2";
export type TriggerSlot = "A" | "B" | "C";
```

### Config 変更案

```ts
export interface Config {
  trajectory: boolean;
  ignore_exe: string[];
  triggerA: GestureTriggerButton;
  triggerB: GestureTriggerButton;
  triggerC: GestureTriggerButton;
  triggerAColor: string;
  triggerBColor: string;
  triggerCColor: string;
  actions: Action[];
}
```

Rust 側も同等に `Config` を拡張する。

### Action 変更案

現状:

```ts
{
  trigger_type: "gesture",
  gesture: "L"
}
```

変更後:

```ts
{
  trigger_type: "gesture",
  trigger_slot: "A",
  gesture: "L"
}
```

要点:

- 同じ `L` ジェスチャーでも `A/B/C` で別 Hotkey を持てる
- trigger button の物理ボタンではなく、ユーザー設定上の slot を action 側の識別子にする
- 実行時は `trigger_slot + gesture` で解決する

## SettingsTab の具体的な追加項目

既存の `表示設定` と `グローバル無視EXE` の近くに、同じ設定枠スタイルで `トリガーボタン設定` セクションを追加する。

### 追加項目

- Trigger A の開始ボタン
- Trigger A の軌跡色
- Trigger B の開始ボタン
- Trigger B の軌跡色
- Trigger C の開始ボタン
- Trigger C の軌跡色

### 開始ボタン候補

- `Right`
- `Middle`
- `XBUTTON1`
- `XBUTTON2`

### 色設定方式

初回は既存 UI に馴染みやすい `select + input type="color"` または単純な `input type="color"` を優先する。

初期値:

- Trigger A: `#FF4D4F`
- Trigger B: `#4C8DFF`
- Trigger C: `#22A06B`

## UI 変更案

### 基本方針

- `SettingsTab` は流用
- `ActionsTab` と `ActionEditor` は、既存一覧・編集体験を残したまま `trigger_slot` を扱えるように最小拡張
- `InfoTab` やアプリタイトルなど、見える名称は `GestureHotkeyApp` へ揃える

### 最小変更で済む見込みの箇所

- `SettingsTab.tsx`
- `SettingsTab.css`
- `ActionList.tsx`
- `ActionEditor.tsx`
- `InfoTab.tsx`
- `index.html`
- `tauri.conf.json`

## 軌跡描画変更案

### 色参照方法

- renderer は実行中の `trigger_slot` を受け取る
- `Config.triggerAColor / triggerBColor / triggerCColor` を参照して描画色を決定する

### 見た目の改善方針

- 現状の細線より見やすい線幅へ変更
- 半透明の本線 + 補助グローを検討
- 先端ドットを追加して現在位置を把握しやすくする
- Trigger A/B/C は色で即時判別できるようにする

### アクション名ラベルの補助表示

- 認識済みジェスチャーに対応する `Hotkey / アクション名` をオーバーレイ表示
- 表示開始: ジェスチャー認識時
- 表示終了: trigger ボタン解放時
- 軌跡レイヤーとは疎結合で持つ

## 改修対象ファイル

### Rust

- `src-tauri/src/config.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/mouse_hook.rs`
- `src-tauri/src/trajectory_renderer.rs`
- `src-tauri/src/action_label_overlay.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`

### TypeScript / React

- `src/types/index.ts`
- `src/store/useStore.ts`
- `src/api/commands.ts`
- `src/components/actions/ActionsTab.tsx`
- `src/components/actions/ActionList.tsx`
- `src/components/actions/ActionEditor.tsx`
- `src/components/settings/SettingsTab.tsx`
- `src/components/settings/SettingsTab.css`
- `src/components/info/InfoTab.tsx`
- `index.html`
- `package.json`

### 配布素材

- `src-tauri/icons/32x32.png`
- `src-tauri/icons/128x128.png`
- `src-tauri/icons/128x128@2x.png`
- `src-tauri/icons/icon.ico`
- `src-tauri/icons/installer-icon.ico`
- `src-tauri/icons/uninstaller-icon.ico`

## 既存設定ファイルとの互換性

### migration 方針

既存 config に新項目が無い場合は以下を補完する。

- `triggerA = "right"`
- `triggerB = "middle"`
- `triggerC = "x1"`
- `triggerAColor = "#FF4D4F"`
- `triggerBColor = "#4C8DFF"`
- `triggerCColor = "#22A06B"`

既存 gesture action に `trigger_slot` が無い場合は `A` を補う。

### validation 方針

- trigger button は `right / middle / x1 / x2` のみ許可
- 色は `#RRGGBB` 形式のみ許可
- 不正値は load 時に fallback

## 3 トリガー化で必要な変更箇所

1. `Config` の trigger/button/color 拡張
2. `Action` の `trigger_slot` 拡張
3. `SettingsTab` に 3 trigger 分の UI を追加
4. `mouse_hook.rs` の trigger 判定を Right 固定から設定参照へ移行
5. `trajectory_renderer.rs` を trigger slot ごとの色参照へ変更
6. action 一意キーを `trigger_slot + gesture` 前提へ変更

## XBUTTON1 / XBUTTON2 の実装可否

- 実装可
- 理由:
  - 既に wheel 系アクション候補として概念が存在する
  - Windows 低レベルマウスフック側で `WM_XBUTTONDOWN / UP` の扱いを追加できる

注意:

- 既存の `x1_button / x2_button` wheel 系アクションと意味が近いため、UI 文言と内部識別子の整理が必要

## 実装手順

1. 名称・配布設定を `GestureHotkeyApp` に統一
2. Config / Action 型を 3 trigger 対応へ拡張
3. migration / validation を追加
4. SettingsTab に trigger button / trail color 設定 UI を追加
5. ActionEditor と一覧表示を `trigger_slot + gesture` 対応へ変更
6. `mouse_hook.rs` を設定参照の trigger 開始へ変更
7. renderer を trigger slot 色参照へ変更
8. アクション名ラベルオーバーレイを仕上げる
9. NSIS インストーラー設定とアイコン反映

## 現在の実装反映状況

- 完了:
  - `GestureHotkeyApp` 名称統一方針の反映
  - Tauri / package / InfoTab への名称反映
  - app / installer / uninstaller icon 生成
  - `Config` の 3 trigger / 3 color 拡張
  - `Action` の `trigger_slot` 拡張
  - `SettingsTab` への Trigger A/B/C 開始ボタン・軌跡色設定 UI 追加
  - `trigger_slot + gesture` キー方針での TS 側 action 一意化
  - Rust 側 config migration / fallback / validation 追加
  - action label overlay の基盤組み込み
- 未確認:
  - Node.js / Cargo 未導入環境のため、Tauri ビルドとランタイム確認
  - NSIS 実生成物のファイル名、Program Files 配置、ショートカット反映
- 継続中:
  - Rust 側 mouse hook / trajectory renderer の実機検証
  - 既存 wheel action との共存確認

## 設定 export / import 追加方針

- `config + gestures` を 1 ファイルの settings bundle として保存する
- import 時は現在設定へ上書きする
- 読込後は Config / gesture 一覧を再取得して即時反映する
- 形式は初回は JSON とし、`formatVersion` を持たせる
- 詳細は `docs/settings-export-import-plan.md` に記載する

## リスクと注意点

- 既存 action キーの変更は UI 一覧、保存、更新、削除の全経路へ影響する
- `XBUTTON1 / XBUTTON2` は既存 wheel 系アクションとの用語衝突に注意が必要
- Tauri 側の名前変更と NSIS 出力名の差分は、最終的にビルド成果物名でも確認する必要がある
- 現在の環境には `node` と `cargo` が入っていないため、実ビルド確認は別途必要

## 2026-06-28 実装反映メモ

### 反映済み

- `SettingsTab` に Trigger A / B / C の開始ボタン設定を追加
- Trigger A / B / C の軌跡色設定を追加
- `Action` を `trigger_slot + gesture` 基準で扱う action key に更新
- Rust 側 `mouse_hook.rs` が Right 固定ではなく設定参照で開始する構成に更新
- `trajectory_renderer.rs` が trigger slot ごとの色を反映する構成に更新
- 認識済みジェスチャーの action label overlay を維持
- settings bundle 形式で export / import を追加

### 今回の重要補足

- installer 方針と整合させるため、release 時の設定保存先は `%APPDATA%\GestureHotkeyApp\` に変更
- `C:\Program Files\GestureHotkeyApp\` 配置後も設定変更できる構成へ修正した

### まだ未確認

- Trigger A / B / C の個別 Hotkey が実 UI / 実入力で成立するか
- XBUTTON1 / XBUTTON2 の実機確認
- export / import の end-to-end 確認
- installer 実インストール後の導線確認
## 2026-06-28 追記: アクション一覧のグループ管理方針を更新

- `Action.group` 文字列を各 action で持って個別編集する案は廃止
- 正式に
  - `Config.groups`
  - `Action.group_id`
  の参照構造へ移行
- group 名編集は `ActionList` の group header 側で行う
- action 新規追加は group 行の `+` から行う
- export / import は `groups + group_id` を含む settings bundle 前提へ更新
- 旧 `group` 文字列ベースの config は migration で吸収する
