# Issue #381 の作業ログ

## 実装者メモ（round 1）

対象 head: 33bf440923866b53cdbe506006991cd83d3bd394

1. 詰まった箇所: `crates/pelway-client-ui/tests/demo_banner.rs`: 「あなただけに表示」の行と msg3 以降の要素が `ElementHandle` の探索で見つからず、`if :` の条件付き要素の後ろの兄弟が落ちる制約と誤診して構造を直しかけた。`i-slint-backend-testing` の走査は `ItemRc::is_visible()` を通り、Flickable のビューポート外にクリップされた行を除外するのが原因だった（窓を 2400px 高にして全行を可視にして解決）。
2. 曖昧だった文: 「全画面に出す常時表示「デモ・模擬応答」をネイティブクライアントの骨組みに付け、人と AI の識別を読み上げ契約どおりにそろえる。」（Issue #381 本文「目的」）。「読み上げ契約どおり」の読み上げ全文の適用範囲が分からず、バッジの読み上げ名（AIメンバー）までをこの Issue の範囲と解釈した。
3. 仮定と解釈: 常時バナーは画面ごとの複製ではなく MainWindow 直下の帯として 1 つ配置した（show-today の切替で消えず「全画面」の契約を満たす最小形）。選ばなかった解釈: TodayView/ChannelView それぞれの内側に置く形（画面ごとに同じ帯を複製し、読み上げにも重複が出うるため）。
4. 足りなかった情報・検証手段: Slint テスト backend が「見つからない」要素を拾わない理由（visibility フィルタ）は crate ソースを読むまで分からず、`docs/` や既存テストのコメントにも記述が無かった。msg3 以降が落ちる症状は条件 `if` と位置が重なっていたため切り分けに debug example（使い捨て、削除済み）を使った。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「よくある失敗」相当の箇所に「Slint のテスト用 backend で要素が見つからないときは `ItemRc::is_visible()` の除外（スクロール外クリップ）を先に疑い、窓を縦長にしてから構造上の仮説を立てる」を足す。

使ったスキル: implement-issue
