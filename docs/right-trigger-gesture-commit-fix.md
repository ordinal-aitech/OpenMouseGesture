# Right Trigger ジェスチャー確定時の通常右クリック抑止修正

## 修正前の症状
- Right Trigger でジェスチャーは描ける
- しかし、描き終わってボタンを離すと通常の右クリックとしても扱われ、コンテキストメニューが出る
- そのため、Hotkey が送られていない、または送られていても右クリックメニューに潰されているように見える

## 原因
- Right Trigger の `ButtonDown` を旧実装で OS / 前面アプリへ先に流していた
- そのため、gesture 成立後にも通常右クリックの文脈が前面アプリ側に残っていた
- pending 状態と gesture 成立状態の境界が曖昧で、gesture 成立後も通常右クリック完了経路へ戻り得る状態だった

## Right Trigger の状態遷移

### 修正後
1. `Right ButtonDown`
   - フック側で pending として保持
   - OS / 前面アプリへはまだ流さない
2. `MouseMove`
   - 押下開始位置からの距離を監視
3. `10px` 未満のまま `Right ButtonUp`
   - 通常右クリックとして `ReplayOriginalClick(...)` を実行
4. `10px` 以上で gesture 成立
   - trail 描画を開始
   - gesture 入力セッションへ移行
5. gesture 成立後の `Right ButtonUp`
   - 通常右クリックには戻さない
   - `GestureCaptured` を発火して Hotkey 実行側へ渡す

## 通常右クリックとジェスチャー成立の分岐条件

### 通常右クリックになる条件
- Right 押下のみ
- Right 押下後、移動量が `10px` 未満
- 終了時点でも click-like 判定

### ジェスチャー成立になる条件
- Right 押下後、移動量が `10px` 以上

## Hotkey 実行が阻害されていた理由
- gesture 成立後にも通常右クリック文脈が残っていたため、`ButtonUp` 時にコンテキストメニューが前面に出ていた
- その結果、ユーザー体感では Hotkey の効果が見えず、未実行に見える状態になっていた

## 修正内容
- `MouseHookService` の trigger 開始処理を整理し、Right を含む全 trigger で `ButtonDown` をいったんフック側に保持するよう変更
- gesture 不成立時のみ `ReplayOriginalClick(...)` で元のクリックを合成再生
- gesture 成立時は通常右クリック再生を行わず、`GestureCaptured` のみを発火
- 旧実装で使っていた forward 状態専用の解放処理は不要になったため削除

## 修正後の挙動
- Right 押下のみなら通常右クリックになる
- Right 押下 + 微小移動なら通常右クリックになる
- Right 押下 + ジェスチャー成立時は通常右クリックメニューを出さず、Hotkey 実行を優先する

## 参照ファイル
- [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:72)
- [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:93)
- [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:201)

## 今後の注意点
- この修正は build 成功まで確認済みだが、物理マウス実機での再確認はまだ必要
- 特に以下を優先確認する
  - Right 押下のみで通常右クリックになること
  - Right 押下 + ジェスチャー成立時にコンテキストメニューが出ないこと
  - Hotkey 実行結果がユーザー体感でも確認できること
