# Issue #411 の作業ログ

## 実装者メモ（round 1）

対象 head: 3f1cd97

1. 詰まった箇所: scripts/tests/conductor-watch-ensure.test.sh: `mise run verify` の並行実行時のみ「systemd: foreign watch present」が失敗する（waitn の 3 秒タイムアウト内に `setsid -f` の watch が pgrep に現れない）。変更なしの main（3884375）でも再現し、単体実行では 2 回とも通ったため、本差分（crates/ 限定）と無関係の負荷依存と判断して進めた
2. 曖昧だった文: 「参照版・integration 版の具体的な型（文字列の版 ID・epoch 数値など）。実装時に v0.7 本文03 §5 と ADR 0009 の用語から決める」（Issue #411 本文「未決事項」）
3. 仮定と解釈: 参照版・integration 版は外部由来の識別子なので `Option<String>`（文字列の版 ID）を採った。u64 epoch は外部の版表現（semver・etag など）を Pelway 内の数値へ写像する仕組みが必要で、既存の外部識別子フィールド（`recipient`・`account`）が `String` であることとも一貫しない。`card_version: u64` は Pelway 内部の楽観排他用 epoch で別物。空文字列の版 ID は「指定なし」（`None`）と同じ意味に正規化し、版が 1 つでも付くときは長さ 0 の区切りを hash に入れて scope 末尾との連結曖昧さを防いだ。選ばなかった解釈: 版を常に hash に put する方式（6 項目の既存 hash が全て変わる）
4. 足りなかった情報・検証手段: v0.7 本文03 §5.1（docs/technical/v0.7/body/03-ai-bot-runtime.md:71）は「参照版」「integration版」を列挙するだけで各項目の意味を定義しない。docs/technical.md:130 と v0.6 由来の `reason-integration-revoked` の用法から「操作が参照する資料の版 ID」「操作が使う外部連携の版 ID」と解釈した
5. 改善案: docs/technical/v0.7/body/03-ai-bot-runtime.md の「固定承認と外部副作用」に「参照版は操作が参照する資料・成果物などの版 ID、integration 版は操作が使う外部連携の版 ID」という 1 文を足すと実装側の解釈が仕様に固定される

使ったスキル: implement-issue
