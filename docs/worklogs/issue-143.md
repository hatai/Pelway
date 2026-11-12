# Issue #143 の作業ログ

## 元の作業（demo-t3/platform-slot9）の各段階と今の main の検査の重複比較

今の main の検査: `mise run verify`（fmt-check / clippy / toolchain-check / hk-validate / cargo test / script-test / hk-check(typos・trailing_whitespace・newlines) / fnox-check / pitchfork-check）、`crates/pelway-http` の `no_body_logs` 試験、`crates/pelway-client-ui` の `no_body_persist` 試験、hk の git hook（pre-commit / pre-push）。

| 段階 | 今の main の対応する検査 | 判定 |
|---|---|---|
| loopback（待受・接続先が 127.0.0.1/::1 だけかの静的走査） | `mise.toml` の `PELWAY_BIND_ADDR = "127.0.0.1:0"`（値の設定はあるが値を検査する仕組みは無い）。`no_body_logs` は 127.0.0.1 での起動を、`http.rs` の `serve_refuses_non_loopback_addresses` は非ループバックの拒否を実行時に確かめるが、`0.0.0.0`・`UNSPECIFIED`・`bind("...")` リテラルをソース全体から探す静的走査は無い | 非重複 → 移植 |
| leak-phrases（禁止語・秘密値の形の走査） | 製品ソースの禁止語走査は無い。`no_body_persist` の `core_forbidden_tokens` は `pelway-client-core` だけを対象に fs-API 名（`std::fs`・`println!` 等）を見る別目的・別範囲の検査 | 非重複 → 移植 |
| external-refs（許可外ホストへの参照の走査） | 外部ホスト参照を走査する検査は無い | 非重複 → 移植 |
| canary（成果物・ログ・端末保存領域の目印走査） | `no_body_logs` が実行時ログの canary 0件を、`no_body_persist` の (a)(c) が端末保存領域の canary 走査と陽性対照を担う（「ファイル・ログを canary で走査する」概念は同じ）。ただし demo-check の canary 段階は `target/` 成果物・`logs/`・`*.log` を `PELWAY_CANARY_*` の目印で受入時に走査し、走査対象・目印・実行時点（テストではなく受入ゲート）が異なる | 一部重複（概念）・検査自体は非重複 → 移植 |
| selftest（違反を入れて検査が失敗することの確認） | `no_body_persist` の陽性対照 (c) はその試験自身の走査関数のもので、demo-check ハーネスの自己テストではない | 非重複 → 移植（積み PR `agent/143-demo-check-selftest` で入れる） |

hk（typos・trailing_whitespace・newlines・cargo fmt/clippy/check）はスタイルと型の検査で、demo-check の各段階とは無関係。

## 移植時の適応（今の main に合わせて変えた点）

- `lib.sh` の除外に追加: `crates/dev-*/`・`crates/pelway-agent-watch/`（ワークスペース manifest の「開発用」区分＝製品ではない crate。agent-watch は見張り役で GitHub 等への正当な外部参照を持つ）、`crates/pelway-client-ui/tokens/`（正本トークンのバイト固定ミラー。トークン名は識別子）、`crates/pelway-http/tests/http.rs`（非ループバック拒否の陰性試験が意図的に違反アドレスを含む）
- `loopback.sh` の `bind("...")` 検査を `::bind("...")`（関連付け関数の形）に絞った。sqlx などのメソッド形式 `.bind("v1")` が `crates/dev-queue-compare` の試験にあり誤検出したため。メソッド形式のソケット bind にリテラルを渡す形は検出しない（既知の限界として loopback.sh のコメントに記載）
- `crates/pelway-at-rest/src/attachments.rs` の `Display` メッセージ「安全な 1 パス成分でない」を「1 パス成分でない」に変更（禁止語リストの `安全`（copy 範囲）に抵触。テストは enum 値を見ていて表示文字列には依存していない）
- wasm32 ターゲットの宣言は移植しない（今の main では demo-check の前提ではない。`target/wasm32-*/` の走査は「存在するものだけ」扱いで、ディレクトリが無ければ何もしない）
- `docs/worklog/dev-infra.md`（旧ブランチの作業記録）は移植しない（今の main の作業記録は `docs/worklogs/` 配下。比較表はこのファイルに書く）

## `mise run verify` への組み込み判断

`demo-check` 本体は verify に含めない。canary 段階は走査対象の成果物が無いと未検証（exit 1）になる設計で、クリーンな checkout（`target/` なし）では必ず失敗するため。またこれはデモ受入ゲートで、コミット前の衛生検査ではない。実行時点: デモ受入前に、製品をビルドしたあと `mise run demo-check`（全段階）。受入モードは `DEMO_CHECK_ACCEPTANCE=1`。
`demo-check` の自己テスト（selftest）は `script-test` 経由で `verify-test` に含めた（積み PR `agent/143-demo-check-selftest`。ハーネス自体の退行を verify が検出できるようにするため。自己テストは一時ディレクトリで完結し成果物に依存しない）。

## 実装者メモ（round 1）

1. 詰まった箇所: 手順 4（テストを先に書く）: 旧ブランチのスクリプトをそのまま走らせると今の main で 4 段階とも失敗し、誤検出（sqlx の `.bind`・トークン名の `safe-`・陰性試験の違反アドレス）と、禁止語リストへの適合違反（`attachments.rs` の「安全な」）の切り分けが必要だった
2. 曖昧だった文: 「重複しない段階だけを、今の main の上の新しいブランチに `mise run demo-check` として入れる」（Issue #143「受け入れ条件」）
3. 仮定と解釈: canary 段階は `no_body_logs`・`no_body_persist` と「canary で走査する」概念が重なるが、走査対象（`target/` 成果物・ログ）と目印（`PELWAY_CANARY_*`）が異なるため「検査自体は非重複」と解釈して全段階を移植した。完全な重複とみなして canary 段階を落とす解釈は選ばなかった
4. 足りなかった情報・検証手段: 製品 crate と開発用 crate の区分は `Cargo.toml` のワークスペース manifest のコメントを根拠にした。`docs/worklog/dev-infra.md` は旧ブランチにだけあり、移植対象かどうか Issue に明記がない（作業記録の置き場所の判断）
5. 改善案: `AGENTS.md` の「Agent workflow」に「旧ブランチの作業記録（docs/worklog/）を移植するときは docs/worklogs/ 配下に書く」と足す文の候補

## 実装者メモ（round 2）

1. 詰まった箇所: 手順 6（pr-size）: 検査本体と selftest を 1 コミットにすると +1192 で上限超過。指揮役の分割案（`<!-- split-plan -->` コメント）で検査本体（PR A）と selftest（PR B・積み PR）に分けた
2. 曖昧だった文: 「PR B は PR A のブランチに積みます」（Issue #143 の split-plan コメント、作成者 hatai）
3. 仮定と解釈: PR B の比較元（base）は `agent/143-demo-check` と解釈し、`gh pr create --base` をそのブランチに向ける。`mise run pr-size` の既定は origin/main との差分なので、PR B の行数確認は `scripts/pr-size.sh --base origin/agent/143-demo-check` で数える解釈を置いた
4. 足りなかった情報・検証手段: 積み PR の `mise run pr-size`（既定 origin/main 比較）が PR A 分も数えて失敗するかどうかは、CI 側の扱いが docs に見当たらず未確認。`--base` で積み元を指定すれば PR B 分だけ数えられる
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「積み PR では `scripts/pr-size.sh --base <積み元ブランチ>` で数える」と足す文の候補

## 実装者メモ（round 3）

1. 詰まった箇所: PR B（`agent/143-demo-check-selftest`）: `script-test` は `&&` でつなぐ既存の 1 行 `run` に selftest を追加する形にした（先の 3 テストが失敗したら selftest は走らない。逐一出すならタスク分割が必要だが、既存の形を変えない判断をした）
2. 曖昧だった文: 「`mise.toml` の `demo-check-selftest` と `script-test` への組み込み、作業記録の追記」（Issue #143 の split-plan コメント「PR B」の範囲、作成者 hatai）
3. 仮定と解釈: 「`script-test` への組み込み」は `script-test` タスクの `run` 行に selftest を追加することと解釈した。`verify-test` の `depends` 自体には触れない（`script-test` 経由で verify に効く）
4. 足りなかった情報・検証手段: selftest の実行時間はこの環境で約 1 分（130 ケース）。verify-test への追加で CI 時間がどれだけ伸びるかは未計測（CI の実測が無い）
5. 改善案: `docs/dev-environment.md` の CI 節に「`script-test` に demo-check selftest が含まれる（`verify-test`）」と足す文の候補
