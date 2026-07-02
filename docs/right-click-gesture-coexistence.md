# Right クリックとジェスチャーの共存

## 修正前の問題
- Right Trigger を使ってジェスチャーを描いたあとでも、通常の右クリック文脈が残り、コンテキストメニューが出ることがあった
- その結果、ジェスチャーに割り当てた Hotkey が体感上は実行されていないように見えた

## 原因
- 旧実装では Right Trigger の `ButtonDown` を先に OS / 前面アプリへ流していた
- そのため、しきい値超過後に gesture が成立しても、通常右クリックの開始状態が前面アプリ側に残っていた
- `ButtonUp` 時にその文脈が完了し、コンテキストメニューが出る余地があった

## 判定しきい値の仕様
- ジェスチャー開始しきい値: `10px`
- 判定対象:
  - trigger 押下開始位置
  - 最新マウス位置
- これらの距離が `10px` 以上になった時点で gesture を開始する

## 通常右クリックとジェスチャー開始の分岐条件

### 通常右クリック
- Right を押す
- 押下中の移動量が `10px` 未満のまま解放する
- または最終的に click-like 判定のまま終了する

処理:
- フック側で保持していた Right クリックを `ReplayOriginalClick(...)` で再生する

### 右ドラッグジェスチャー
- Right を押す
- 押下中の移動量が `10px` 以上になる

処理:
- gesture を開始する
- 以後は通常右クリック経路へ戻さない
- `ButtonUp` 時は gesture 完了として扱い、Hotkey 実行側を優先する

## 修正内容
- Right だけ特別扱いせず、Trigger 開始時は `ButtonDown` をすべてフック側で保持する構成へ統一
- `MouseHookService` では、gesture 不成立時だけ `ReplayOriginalClick(...)` を呼ぶ
- gesture 成立時は `GestureCaptured` を発火して終了し、通常右クリックを再生しない
- これにより、pending 状態と gesture 成立状態をコード上で明確に分離した

## 修正後の挙動

### 期待する挙動
- Right 押下のみでは通常右クリックになる
- Right 押下 + 微小移動では通常右クリックになる
- Right 押下 + しきい値超過で gesture 成立時は、コンテキストメニューを出さず Hotkey 実行を優先する

### 実装上の参照箇所
- Trigger 開始と抑止:
  - [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:72)
- click-like 判定と通常クリック再生:
  - [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:93)
- 通常クリック再生処理:
  - [MouseHookService.cs](C:/Users/ohkat/OneDrive/ドキュメント/Windowsアプリ開発/src/GestureHotkeyApp/Services/MouseHookService.cs:201)

## 今後の調整ポイント
- 物理マウス実機で、Right 押下のみの自然さを再確認する
- アプリごとの右クリック挙動差がないかを確認する
- `10px` が厳しすぎる、または緩すぎる場合は設定値の再検討余地がある
