# report.md — Open Mouse Gesture: 残存する斜め軌跡ジッターの根本修正

## 対象

Working directory: `C:\GitHub\open-mouse-gesture`
Primary source: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`
実行: Claude Code CLI, Sonnet, medium effort。

前タスク（コミット `8b47b41`, デザインB + mutex統合修正）に続くタスク。
ユーザーの実機確認により、色/太さ/水平・垂直ストロークは改善したが、
**斜めストロークのみ依然として明確に震える**ことが報告された。本タスクはこの
残存ジッターの根本原因特定と修正のみを対象とする。レガシー設定インポートの
意味的な再構築はスコープ外（ユーザーが手動再設定を受け入れ済み）。

## 参照した正本

- `C:\GitHub\ai-executor\AGENTS.md`
- `C:\GitHub\ai-executor\00_ROUTING.md`
- `C:\GitHub\ai-executor\01_AI_ORCHESTRATOR_RUNTIME.md`
- `C:\GitHub\ai-executor\02_SOFTWARE_IMPLEMENTATION_EXECUTION.md`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/README.md`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/CLAUDE.md`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/docs/*.yaml`
- `git log` 直近コミット `8b47b41`, `40caac7`
- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`（全文読了）
- `.../src-tauri/src/mouse_hook.rs`（座標の発生源: `mouse_data.pt.x/y`）
- `.../src-tauri/src/lib.rs`（`append_trajectory_point`/`update_trajectory`/`clear_trajectory_display` の呼び出し経路）

参照できなかった正本はなし。

## 斜めジッターの正確な根本原因

**座標そのものは常に正しく、不変（immutable）だった。** マウスフックから来る
`mouse_data.pt.x/y`（スクリーン絶対座標の整数）がそのまま `append_trajectory_point`
経由で `TRAJECTORY_STATE.points` に追記されるだけで、DPIスケーリングや別々の丸め、
リトロアクティブな平滑化は一切存在しない。X/Y間の丸めルール差も無かった。
つまり「点そのものが動く」問題ではなかった。

問題は **レイヤードウィンドウ自体が毎フレーム動いていたこと** にあった。

- 旧 `compute_bounds()` は、その時点の全点列の `min_x/min_y/max_x/max_y` を
  そのままウィンドウの `left/top/right/bottom` として使っていた。
- `ensure_window_matches_bounds_with_snapshot()` は `left/top` が前回と異なれば
  `SetWindowPos` で **実際にウィンドウを移動**させ、その後 `render_to_memory_dc_with_snapshot`
  → `UpdateLayeredWindow` で再描画していた。
- 水平・垂直に近いストロークは手ぶれが片方の軸にしか及ばないことが多く、
  `min_x`（またはmin_y）のどちらか一方だけが安定するため、ウィンドウは
  「リサイズのみ」で済むフレームが多かった（=移動しない=震えにくい）。
- **斜めストロークは手ぶれがX/Y両軸に同時に及びやすく**、`min_x`・`min_y`の
  どちらか（または両方）がフレームごとに変化しやすい。これが起きるたびに
  `SetWindowPos` でウィンドウの `left/top` が実際に動いていた。
  `SetWindowPos`（移動）→`UpdateLayeredWindow`（再描画）の間に一瞬でも
  デスクトップコンポジタが古いビットマップを新しい位置で（またはその逆で）
  合成する瞬間が生じると、**既に描画済みの軌跡全体が画面上で一瞬ズレて見える**。
  これが「水平/垂直は改善したが斜めだけ明確に震える」という報告と正確に整合する。

### 前回のmutex統合修正がなぜ不十分だったか

前回の修正（`8b47b41`）は「点列スナップショットとバウンディングが別々のロックで
不整合になる」という問題（*同一フレーム内でのデータ不整合*）を解消した。これは
正しく必要な修正だったが、**バウンディングが確定した後の「ウィンドウが毎フレーム
移動する」という構造自体は変えていなかった**。つまり「不整合なジオメトリを
描画する」バグは直したが、「整合したジオメトリであっても、ウィンドウの
物理的な移動自体が視覚的な段差を生む」という別の原因は未修正のまま残っていた。
これが斜めストロークでのみ顕著に残存した理由である。

## 座標空間と丸めルール（変更前/変更後で不変）

- 記録座標空間: スクリーン絶対座標（`i32`、マウスフックの生値）。**変更なし**。
  一度 `TRAJECTORY_STATE.points` に追記された点は書き換えられない（不変）。
- ローカル描画座標: `local = point - window_origin`。`window_origin` は常に整数。
- X/Y は常に同じ式・同じ丸め規則（`i32`の単純な減算、四捨五入なし）で扱われ、
  軸ごとの非対称な丸めは存在しない（変更前後とも）。
- DPI変換: マウスフックは物理ピクセル座標を返すため、フレームごとに変動する
  DPIスケール変換は元々存在しない（確認済み、変更なし）。

## ウィンドウorigin/挙動: 変更前 → 変更後

- **変更前**: `compute_bounds(points)` = 現在の全点列から都度計算した
  タイトな矩形（+マージン）。ジェスチャーの手ぶれで `min_x`/`min_y` が
  変化するたびに、ウィンドウの `left/top` が実際に動く。
- **変更後**: ジェスチャー開始点まわりに `GESTURE_PREALLOC_RADIUS = 500px` の
  余白を持つ矩形を一度だけ確保（**anchor**）。以後は「タイトな実バウンディング
  ∪ anchor」を取るだけなので、実際の軌跡が事前確保領域に収まっている間は
  **矩形が一切変化しない**（=`SetWindowPos`による移動が一度も起きない）。
  事前確保領域を超えて伸びた場合のみ、矩形は外側へ単調拡大する（縮小はしない
  = 既に描画済みの点が再クリップされることはない、という既存の保証を維持）。
  ジェスチャー終了（`clear_trajectory_display`／非表示化）時に anchor を
  明示的にリセットし、次のジェスチャーが別の場所で始まっても前回の位置を
  引きずらないようにした。

## 実装した修正

対象ファイル: `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`

1. `GESTURE_PREALLOC_RADIUS: i32 = 500` を追加。典型的なジェスチャーの
   移動範囲を十分に覆う値として選定（ジェスチャー中にウィンドウが
   一切動かないことを狙う）。
2. 既存の `compute_bounds()`（タイトなバウンディング計算、既存テストごと維持）は
   そのまま残し、新たに純粋関数 `merge_gesture_rect(anchor: Option<RECT>, points) -> Option<RECT>`
   を追加。anchorが無い場合は先頭点まわりに事前確保矩形を新規作成、ある場合は
   タイトな実バウンディングとの和集合を返すのみ（副作用なし、決定的、
   単体テストで直接検証可能）。
3. レンダースレッド専用の状態付きラッパー `compute_window_rect(points)` を追加し、
   `static GESTURE_ANCHOR: Mutex<Option<RECT>>` を介して `merge_gesture_rect` を
   呼び出す。点列が空になった時点で anchor を `None` にリセットする。
4. `window_proc` の `WM_UPDATE_TRAJECTORY`（`visible=true`）ハンドラで、
   バウンディング取得を `compute_bounds(&snapshot_points)` から
   `compute_window_rect(&snapshot_points)` に置き換え。
5. `visible=false`（非表示化）分岐でも `GESTURE_ANCHOR` を明示的にリセットし、
   次回ジェスチャーが前回位置の矩形を引きずらないようにした。
6. 3レイヤー（glow/body/core）は従来どおり `render_to_memory_dc_with_snapshot` 内で
   同一の `offset`・同一の `snapshot_points` を使って `draw_stroke_mask` を
   3回呼ぶ構造を維持（変更なし）。これにより新しい `compute_window_rect` の
   出力を使っても3層のジオメトリが完全に一致することは構造的に保証される。

デザインB（コア/ボディ/グローの色・太さ・アルファ）は一切変更していない。
`ACTION_LABEL_OVERLAY_ENABLED` は本タスクで触れていない。

## 追加テスト

すべて `trajectory_renderer.rs` 内の `#[cfg(test)] mod tests` に追加（既存25テストは無変更で維持）。

1. `merge_gesture_rect_is_none_for_empty_points` — 空点列は常に `None`。
2. `merge_gesture_rect_first_call_anchors_around_first_point` — 開始点まわりに
   `GESTURE_PREALLOC_RADIUS` 分の矩形が生成されることを検証。
3. `merge_gesture_rect_origin_stays_fixed_for_diagonal_motion_within_radius` —
   X/Y両軸に手ぶれを伴う斜め移動50点を模した点列でも、事前確保範囲内である限り
   矩形が1バイトも変化しないことを検証（**斜めジッター修正の直接証拠**）。
4. `merge_gesture_rect_diagonal_up_right_and_down_right_are_equally_stable` —
   up-right/down-right どちらの斜め方向でも同一の式・同一の安定性であることを検証
   （軸ごとの非対称な丸めが無いことの根拠）。
5. `merge_gesture_rect_expansion_beyond_anchor_never_shrinks_or_reclips` —
   事前確保範囲を超えて伸びるケースでも単調拡大のみで縮小しないことを検証
   （＝既に描画済みの点が再クリップされない、という要件の証明）。
6. `merge_gesture_rect_is_deterministic_regardless_of_call_pattern` —
   逐次構築でも一括構築でも最終的な矩形が同一になることを検証（決定性）。
7. `compute_window_rect_resets_anchor_when_points_become_empty` —
   ジェスチャー終了後に anchor がリセットされ、次のジェスチャーが別の場所で
   始まっても前回位置を引きずらないことを検証（クリア/リセットの回帰防止）。

水平・垂直ストロークの安定性は既存の `compute_bounds_*` 系テスト（変更なし）で
引き続き担保される。3レイヤーの幾何一致は、`render_to_memory_dc_with_snapshot`
が単一の `offset`/`snapshot_points` を3回とも使う構造そのもの（コードレベルの保証）
によるものであり、GDIハンドルに依存するため単体テストの対象にはしていない
（Win32 GDI呼び出しをモックしない限りテスト不可能なため）。

## テスト結果

```
cargo test --lib trajectory_renderer
running 17 tests ... test result: ok. 17 passed; 0 failed

cargo test （フルスイート）
running 39 tests ... test result: ok. 39 passed; 0 failed; 0 ignored
```

既存テストの破壊なし。新規追加7テストすべて成功。

## ビルド結果

- `npm run build`（フロントエンド, `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/`）: 成功。
- `npm run tauri build`（同ディレクトリ）: 成功。
  `target\release\GestureHotkeyApp.exe` および
  `target\release\bundle\nsis\GestureHotkeyApp_0.1.0_x64-setup.exe` を生成。
  警告は本修正と無関係の既存warning（未使用`mut`2件、非snake_caseフィールド/引数、
  いずれも変更前から存在）のみで、エラーなし。
- `npm run dist:windows`（リポジトリルート）: 成功。
  `dist/windows/OpenMouseGesture-x64.exe`（12,834,304 bytes,
  sha256=b642649ed416e48bbb449a116e4e46d2661a14c1f9105628698ccafc3ca7f553）
  `dist/windows/OpenMouseGesture-Setup-x64.exe`（2,796,231 bytes,
  sha256=9c66e7ee5539454e487f50aa92ae7205ef5e1530eb8a2277d34dcb6752b1325a）
  `SHA256SUMS.txt`, `build-info.json`（commit=`40caac71db78ee674a12fa7b076d5134f42829f4`、
  本タスクのコミットはビルド後に作成したため未反映。再現性が必要な場合は
  `npm run dist:windows` を再実行すればSHAが更新される。安全な操作だが今回は未実施）。

## 回帰レビュー

- 右クリックの短時間コンテキストメニュー通過、右ドラッグジェスチャー、
  Middle/X1/X2トリガー、キーボードトリガー、Trigger A/B/C永続化、トレイ挙動、
  フック解放: `trajectory_renderer.rs` 以外のファイルは一切変更していないため、
  これらのロジックへの影響なし。`cargo test`フルスイート（`mouse_hook`関連含む
  39テスト）が全通過していることでも裏付け。
- `ACTION_LABEL_OVERLAY_ENABLED = false`: 未変更。
- デザインB（色/太さ/アルファ定数）: 未変更。3レイヤーの描画ロジック
  （`draw_stroke_mask`呼び出し3回）も未変更。
- `dist/windows/` エクスポート: 成功、既存フォーマットのまま。
- 既存の `compute_bounds` 単調拡大保証（既にクリップされない）: 維持し、
  新しい `merge_gesture_rect` も同じ不変条件を継承（テスト5で検証）。

## 物理確認

`dist\windows\OpenMouseGesture-x64.exe` はポータブル実行ファイルで管理者権限を
要求しないため、UACなしで直接起動できた（`Start-Process` で起動、PID確認済み、
プロセスは起動中のまま残してある）。ただし実際のジェスチャー描画は
トリガーボタンを押しながらマウスを動かす必要があり、レガシー設定インポートで
トリガー割り当てが破損している可能性があるため、自動化での再現・スクリーンショット
検証はスコープ外かつ信頼できないと判断し、実施していない。

**残っている物理確認（1件のみ）**:
現在起動中の `OpenMouseGesture-x64.exe` （またはトレイから再起動）で、
設定済みのトリガーを使って以下を確認してください。
1. ゆっくり右下方向に斜め線を描く
2. ゆっくり右上方向に斜め線を描く
3. 斜め区間を含むジグザグを描く
4. すでに描画済みの部分が固定されたままで、震えないことを目視確認する

## 変更ファイル

- `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/src/trajectory_renderer.rs`（唯一のソース変更、+208/-4行）
- `report.md`（本ファイル、上書き更新）

pre-existing だった `source-v1.0.1/7-rate-OpenMouseGesture-b8f5357/src-tauri/Cargo.toml` と
`.../license.html` の未ステージ変更（作業開始前から存在、行末変換の警告のみ）は、
本タスクと無関係のため**変更もコミットもしていない**。
既存の設定ファイル（`config.json`/`gestures.json`）とそのバックアップは一切触れていない。

## Git

- コミットSHA: （このセクションはコミット後に確定値へ更新）
- Push先: `origin/main`
- 最終 `git status`: （コミット後に追記）

## 未検証事項

- 実機での目視確認（上記「残っている物理確認」参照）。特に斜めストロークで
  震えが実際に解消して見えることは、ユーザーの目視確認が必要（自動テストは
  ウィンドウ移動が発生しないことをロジックレベルで証明しているが、
  最終的な視覚的知覚はユーザー確認が必要）。
- `GESTURE_PREALLOC_RADIUS = 500px` が実際の使用パターン（画面解像度、
  ジェスチャーの典型的な振れ幅）に対して十分かどうかは、実機確認前は推測。
  もし特定の環境で500pxを超える斜めジェスチャーが頻発する場合、その区間だけ
  矩形拡大に伴う一度きりの再配置が発生しうる（既存の「単調拡大」保証により
  既描画点のクリップは起きないが、拡大の瞬間に一度だけウィンドウ移動が
  発生する可能性はゼロではない）。

## 判断待ち事項

`User decision required: none`

## 最終ステータス

**Paused for one physical action**: 斜めジッターの根本原因特定（ウィンドウが
毎フレーム移動していたこと）、修正実装（ジェスチャー単位の固定anchor）、
新規テスト7件、フル回帰テスト（39件）、フロントエンド/Tauriビルド、
`dist/windows` 再生成はすべて完了・成功。ユーザーによる実機での斜め線
目視確認（上記1件）のみ残課題。
