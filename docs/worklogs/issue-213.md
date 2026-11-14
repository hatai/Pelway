# Issue #213 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `crates/pelway-schedule/tests/schedule.rs`（`dom_and_dow_default_is_posix_or`）: 2026-11-21 からの次回発火を 12-13 と書いたが、POSIX OR では金曜 11-27 が先に来る。実装ではなく期待値のカレンダー誤りで、試験が誤りを検出した。12-12 起点→12-13（日曜）に直して OR 意味を固定した。
2. 曖昧だった文: 「Asia/Tokyo での解釈、DST の無い前提での境界ケース（月末、うるう年、曜日併記）のテスト観点が定義されている → #183 のテスト分類または該当 PR で確認」（Issue #213 受け入れ条件）。#183 の分類文書がリポジトリに存在しない（Issue は ready-for-agent のまま）ため「該当 PR」側を選び、ADR 0010 の検証観点表と `tests/schedule.rs` で定義した。
3. 仮定と解釈: 「依存を追加する PR または選定文書」は排他でなく両方を採った（`pelway-schedule` 新設で依存を固定し、選定理由は ADR 0010）。評価タイムゾーンは `Schedule` に焼き込まず `DateTime<Tz>` の引数で渡す解釈を選び、Asia/Tokyo 固定 API は選ばなかった（将来テナント・ユーザ別 TZ が要る可能性と、試験で TZ を注入できることから）。日付と曜日の併記の既定は POSIX OR とし、croner の `+` 前置き・`dom_and_dow` は使わない。
4. 足りなかった情報・検証手段: croner の MSRV（`cargo info` で rust-version が unknown）。mise の rust 1.99.1 で実ビルド・実試験して代替した。#183 の分類文書の想定置き場（テスト計画の独立文書か v0.7 検証文書か）も未決のまま。
5. 改善案: `.agents/skills/write-issue/SKILL.md`の「受け入れ条件の書き方」に「確認先として別 Issue の成果物（文書・表）を指す場合、その Issue が未実施なら確認先が存在しない旨を本文に書くか、代替の確認場所を併記する」を足す。

使ったスキル: implement-issue、`.agents/skills/tdd/SKILL.md`（seam: `pelway_schedule::Schedule` の公開 API。`tests/schedule.rs` の結合試験で `parse`/`next_after` を固定。`from` を引数注入するため時刻のモックは不要と判断）、write-pull-request
