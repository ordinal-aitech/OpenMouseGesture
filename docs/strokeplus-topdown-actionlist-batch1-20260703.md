# StrokePlus.net -> ActionList 直接対応メモ（Batch 1 / 2026-07-03）

目的:

- StrokePlus.net の設定を、現在の OpenMouseGesture ベース実装の `ActionList` へ直接入れられる形で上から順に対応させる
- 今回はまず、**現在の 7 ジェスチャー x 3 Trigger Slot = 21 枠** に収まる分を `Batch 1` として整理する

前提:

- 現在の gesture テンプレートは以下の 7 個
  - `左`
  - `右`
  - `下`
  - `上`
  - `下→右`
  - `上→下`
  - `G`
- Trigger Slot は `A / B / C` の 3 系統
- よって、gesture action として同時に持てる枠は `21`
- StrokePlus.net 側の「文字列送信」2件は現状未対応
- `最大化または復元` は現状アプリ側では `window_operation = maximize` まで

## 方針

- ユーザー要望どおり、**上から順に** ActionList へ入れる
- 未対応ジェスチャー形状は、今回は一旦 **空いている既存 gesture 枠へ順番に割り当てる**
- つまり今回は
  - 元ジェスチャー形状の厳密再現より
  - ActionList へ直接投入できること
  を優先する

## Batch 1 対応一覧

| No | 元カテゴリ | StrokePlus 名 | 元操作 | ActionList 仮割当 | 実行内容 | 備考 |
|---|---|---|---|---|---|---|
| 1 | クリップボード | カット | `/ Down / Up` | `A + 左` | `Ctrl+X` | 直接可 |
| 2 | クリップボード | コピー | `/ Down` | `A + 右` | `Ctrl+C` | 直接可 |
| 3 | クリップボード | シフトデリート | `\ Down \ Up` | `A + 下` | `Shift+Delete` | 直接可 |
| 4 | クリップボード | デリート | `\ Down` | `A + 上` | `Delete` | 直接可 |
| 5 | クリップボード | ペースト | `/ Up` | `A + 下→右` | `Ctrl+V` | 直接可 |
| 6 | クリップボード | リネーム | `\ Up` | `A + 上→下` | `F2` | 直接可 |
| 7 | タブ | 開く | `Up` | `A + G` | `Ctrl+T` | 直接可 |
| 8 | タブ | 進む | `Right` | `B + 左` | `Ctrl+Tab` | 直接可 |
| 9 | タブ | 閉じたタブを開く | `Up Down` | `B + 右` | `Ctrl+Shift+T` | 直接可 |
| 10 | タブ | 閉じる | `Down` | `B + 下` | `Ctrl+W` | 直接可 |
| 11 | タブ | 戻る | `Left` | `B + 上` | `Ctrl+Shift+Tab` | 直接可 |
| 12 | 全般 | ウィンドウ切替 | `Right Up` | `B + 下→右` | `window_operation=maximize` | 復元トグルは未再現 |
| 13 | 全般 | ウィンドウ選択 | `Up` | `B + 上→下` | `Win+Tab` | 直接可 |
| 14 | 全般 | やり直し | `Up Right` | `B + G` | `Ctrl+Y` | 直接可 |
| 15 | 全般 | 検索 | `S` | `C + 左` | `Ctrl+F` | 暫定 gesture |
| 16 | 全般 | 元に戻す | `Up Left` | `C + 右` | `Ctrl+Z` | 暫定 gesture |
| 17 | 全般 | 更新 | `O` | `C + 下` | `F5` | 暫定 gesture |
| 18 | 全般 | 最小化 | `Down` | `C + 上` | `window_operation=minimize` | 直接可 |
| 19 | 全般 | 進む | `Right` | `C + 下→右` | `Alt+Right` | 直接可 |
| 20 | 全般 | 先頭 | `Left Up` | `C + 上→下` | `Ctrl+Home` | 直接可 |
| 21 | 全般 | 全画面化 | `Right Down` | `C + G` | `F11` | 直接可 |

## Batch 1 の投入結果イメージ

- Trigger A:
  - クリップボード 6件
  - タブ 1件
- Trigger B:
  - タブ 4件
  - 全般 3件
- Trigger C:
  - 全般 7件

## Batch 1 に入らなかった項目

現在の `21` 枠を超えるため、次バッチ扱いにする項目です。

### 次バッチ候補

| StrokePlus 名 | 元操作 | 実行内容 | 状態 |
|---|---|---|---|
| 全選択 | `^` | `Ctrl+A` | Batch 2 |
| 末尾 | `Left Down` | `Ctrl+End` | Batch 2 |
| 戻る | `Left` | `Alt+Left` | Batch 2 |

### 現状未対応

| StrokePlus 名 | 元操作 | 内容 | 状態 |
|---|---|---|---|
| 個人メールアドレス | `M` | 文字列送信 `ohka.type11@gmail.com` | 現状未対応 |
| 仕事メールアドレス | `W` | 文字列送信 `watakou0604@gmail.com` | 現状未対応 |

## 追加メモ

- `Batch 1` をそのまま `config.actions` に入れられるよう、JSON 断片を `artifacts/strokeplus-topdown-actions-batch1.json` として出力する
- これはあくまで **暫定の直接投入版** で、gesture 形状の意味対応までは保証しない
- 後で gesture テンプレート数を増やせば、より元設定に近い配置へ並べ替え可能
