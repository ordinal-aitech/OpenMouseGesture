# ジェスチャー開始時のちらつき修正

## 修正前の症状

- ジェスチャー開始瞬間に画面が一瞬止まる / ちらつく
- 新しいジェスチャーを始めたときに、前回軌跡が一瞬だけ再表示される

## 原因

- overlay の表示切り替えと trail 描画開始が同じタイミングで走っていた
- 前回完了時の clear timer と、次回開始時の描画更新がセッション境界なしで同居していた
- 古い描画更新を「前回のもの」と識別して捨てる仕組みがなかった

## 前回軌跡が見えていた理由

- 前回の完了後に残っているフェード待ち状態または pending update が、新規開始時の render パスへ紛れ込んでいた
- clear より前に旧フレーム相当の描画が通る可能性があった

## ちらつき / 停止感の原因

- overlay の `Show()` / `Hide()` の切り替え
- 開始時 clear と新規 render の順序不定
- 開始時に古いセッションを切り捨てる優先処理がなかったこと

## 修正内容

- overlay を常駐再利用方式へ変更
- `SessionId` を導入して、ジェスチャー単位で更新を分離
- `GestureTrailUpdateKind`
  - `Start`
  - `Update`
  - `Complete`
  - `Clear`
  を導入
- `Start` 時は以下を必ず実施
  - clear timer 停止
  - clear session 破棄
  - 現在描画の即時クリア
  - active session 更新
- `Start` 更新は `DispatcherPriority.Send` で優先処理
- 古い session の update は描画しない

## 修正後の開始シーケンス

1. Trigger 押下
2. pending 状態
3. しきい値超過
4. `Start(sessionId)` 発行
5. 前回状態を clear
6. 今回 session を active 化
7. 新規軌跡描画開始
8. `Update(sessionId)` を追記
9. `Complete(sessionId)` で終了
10. fade 後に current session のみ clear

## 今後の調整ポイント

- 実機で開始時停止感が消えたかの再確認
- まだ停止感が残る場合は overlay bounds 更新頻度の最適化
- まだ旧軌跡が見える場合は start 直前 clear のさらなる軽量化
