# Trigger ボタン直接入れ替え修正

## 問題

- Trigger A / Trigger B は重複不可の前提だった
- そのため UI 上で片側だけ先に変更すると、一時的に同じボタンになる組み合わせが弾かれていた
- 具体的には、以下のような直接変更ができなかった
  - `Trigger A = Right`, `Trigger B = Middle`
  - ここから `Trigger A = Middle`, `Trigger B = Right`

## 原因

- `MainWindowViewModel` の `TriggerAButton` / `TriggerBButton` setter が、変更対象だけをそのまま `TryUpdateTriggerButtons(...)` に渡していた
- その結果、片側変更の途中状態が「重複」と判定され、直接スワップとして扱われていなかった

## 修正内容

- 片側の選択値が「もう片方の現在値」と一致した場合は、単純な重複ではなくスワップ操作として扱うよう変更した
- 実装上は ViewModel 側で更新ペアを組み替え、コントローラーには最終状態をまとめて渡すようにした

## 修正後の挙動

- A 側で `Middle` を選ぶと、B 側は自動で元の A 値へ入れ替わる
- B 側で `Middle` を選ぶと、A 側は自動で元の B 値へ入れ替わる
- XButton1 / XButton2 を経由しなくても、Right と Middle をそのまま入れ替えられる

## 2026-06-26 の確認結果

PowerShell の UI Automation で実アプリの ComboBox を操作し、`config.json` の保存値を読み取って確認した。

### 確認 1

- 開始状態:
  - `TriggerAButton = Right`
  - `TriggerBButton = Middle`
- 操作:
  - Trigger A の ComboBox で `Middle` を選択
- 結果:
  - `AFTER_FIRST=A:Middle B:Right`

### 確認 2

- 開始状態:
  - `TriggerAButton = Middle`
  - `TriggerBButton = Right`
- 操作:
  - Trigger B の ComboBox で `Middle` を選択
- 結果:
  - `AFTER_SECOND=A:Right B:Middle`

## 補足

- 今回の確認は UI からの直接変更で行った
- 一時的に同一値へ変えてから別値へ直す回避操作は不要になった
