# 要件変更サマリー

作成日: 2026-06-23
対象: `requirements-draft.md` -> `requirements-draft-v2.md`

## 変更の要点

1. 対応OSを Windows 11 専用に変更
2. UI言語を日本語に変更
3. アプリの目的を「マウスジェスチャーで Hotkey を送信する専用ツール」に限定
4. 2系統の開始ボタン設定を追加
5. 左ボタン開始を使用不可に変更

## 削除した要件

以下の汎用コマンド要件を削除した。

- RunProgram
- OpenUrl
- OpenMail
- Window maximize/minimize/next/prev
- Delay など Hotkey 以外の汎用コマンド
- 外部 DLL プラグイン
- Lua scripting
- Win32 message send/post
- OSD の高度機能
- マルチモニタ専用操作
- Password 専用コマンド

## 追加した要件

### Trigger A / Trigger B

- 開始ボタンを 2 系統持てる
- 各系統で開始ボタンを個別設定できる
- 同一の開始ボタンを両系統に設定できない
- 同一ジェスチャー形状でも系統ごとに別 Hotkey を割り当てられる

### 左ボタン禁止

- 左ボタンは開始ボタン選択肢に表示しない
- 左ボタンは gesture 開始入力として扱わない

### 日本語UI

- 初版から日本語 UI を前提とする
- 画面名、設定項目、通知文を日本語で設計する

## 仕様の方向性の変化

旧ドラフト:
- 汎用ジェスチャー操作アプリ
- 複数コマンド種別あり
- 拡張性重視

新ドラフト:
- Hotkey 送信専用アプリ
- 目的特化型
- 安定性と軽量性重視

## 実装への影響

- Command Engine は SendHotkey のみに絞れる
- 設定 UI が単純化される
- テスト対象が減る
- MVP の完成を早めやすい
- Trigger A / Trigger B の競合管理が新たな実装ポイントになる
