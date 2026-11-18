# Issue #385 の作業ログ

## 実装者メモ（round 1）

対象 head: 1370a86（実装コミット f1e32d6 の push 後・署名付け替え後の SHA）

1. 詰まった箇所: `crates/pelway-client-ui/tests/timeline_virtualization.rs`: ListView の行の y は推定アンカー（`anchor_y = cached_item_height * offset`、i-slint-core `model/repeater.rs`）で付け替わるため、スクロール前後で記録した y をそのまま比べると「隣接行の y 差 = 高さ」が成立しない。実体化中の行を要素 API（`ChannelView::timeline-row` の id）で直接測る形に直した。probe は固定高さなのでこのずれが出ない。
2. 曖昧だった文: 「隣り合う生成行の y 差がその間の高さの合計と 0.5px 未満で一致し、抜けが無い」（Issue #385・受け入れ条件）: 「同時に実体化している隣接行の実ジオメトリで確かめる」と解釈した（推定アンカー含みの記録値では測れない）。
3. 仮定と解釈: `TimelineEntry` に `DecisionCard` マーカーを足した（時系列にカードの位置が無く、仮想化でも並びを保つには項目化が必要。中身は `data-decision-*` を行が読む）。`MessageEntry` に時刻の欄がないため `time` は空のままにした。
4. 足りなかった情報・検証手段: Slint ListView が「見えていない行の高さを平均値で推定して絶対位置を決める」ことは `model/repeater.rs` を読むまで分からず、記録値のずれの原因特定に時間がかかった。
5. 改善案: `docs/research/slint-tauri-client-measurements.md` の仮想化の節に「行の絶対位置は推定値で、連続する実体化行同士の並びだけが実高さで保証される」を足す。

使ったスキル: implement-issue, write-pull-request

## 実装者メモ（round 2）

対象 head: 7f493aa（修正コミット 1b177fd の push 後・署名付け替え後の SHA）

1. 詰まった箇所: `crates/pelway-client-ui/src/bridge.rs` の `on_open_decision_chat`: 判断待ち一覧からチャットへ戻る台帳駆動の流れ（Issue 331）では `timeline-items` を誰も設定しないため、`data-decision-visible` が true でも `decision-card` 行が無く判断カードが実体化しない退行があった（round 1 のレビュー指摘）。`ensure_decision_card_row` を足し、カード行が無ければ末尾に置く。
2. 曖昧だった文: 「既存のカード（判断、引き継ぎ、社外送信、私信）がタイムライン上で同じ種類として出る」（Issue #385・受け入れ条件）: 「`apply_channel_view` で渡した時系列に限る」か「台帳駆動の既存の結線でも出る」かは本文に書かれていない。後者と解釈し、`pending_decisions.rs` に `DecisionCardView` の要素 API 検査を足した。
3. 仮定と解釈: この骨組みの流れの時系列は空が自然（チャンネル内容は未結線）。`timeline-items` を持たない経路では末尾にカード行を置くフォールバックにした（`apply_channel_view` で時系列が渡る場合はその中の `decision-card` 行が使われ、フォールバックは発動しない）。
4. 足りなかった情報・検証手段: 「チャット内の判断カードは `data-decision-*` だけで出る」という仮想化前の前提を壊していたのを、既存試験（プロパティの値だけを見る `pending_decisions.rs`）では検出できなかった。要素 API で実体化を見る検査が要る。
5. 改善案: `crates/pelway-client-ui/tests/pending_decisions.rs` の doc コメントに「`data-*` の値だけでなく要素の実体化も確かめる」を足す。

使ったスキル: implement-issue
