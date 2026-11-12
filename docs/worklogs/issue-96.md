# Issue #96 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 手順6（mise run pr-size）: 実装と検証一式で追加 +874 行（Cargo.lock は linguist-generated で除外）となり、上限 800 を超えた。テスト・検証・説明を削らず重複の除去（in-process テスト間の同一 assertion の統合、ADR 0004 の根拠説明を lib.rs・Cargo.toml の正本へ集約、echo バイナリの未宣言ツール削除）を試んだが、-74 行には届かなかった。ほか: rmcp 3.9.0 の `Tool`・`ToolAnnotations` は `#[non_exhaustive]` のため構文リテラルで作れず `Tool::new(..).with_annotations(ToolAnnotations::from_raw(..))` を使う。`#[tokio::main]` は既定で `rt-multi-thread` を要求するため `flavor = "current_thread"` 指定にした。
2. 曖昧だった文: 「gatewayの中でMCPサーバーに接続し、ツールの一覧、annotations、呼び出しを扱えるか試す」（Issue #96 の「やること」）。gateway に該当するコードがまだ存在せず、「試す」結果をコードとテストで残すのか文書で残すのかが書かれていない。
3. 仮定と解釈: gateway は architecture-v0.7.md の「連携gateway（GW）」枠として新規 crate `pelway-gateway` を立て、rmcp `=3.9.0` を依存に入れて接続・一覧・呼び出しの詰め替えとテストをコードで残すと解釈した（issue #94 と違い、こちらは検証対象がライブラリそのものなのでコードが成果物になる）。接続は 02 の「本体の既定toolは国内ローカルstdioのみ許可」案に従い stdio 子プロセスを本線とし、テストでは公式 rust-sdk と同じ in-process duplex（`tokio::io::duplex`）も使う。選ばなかった解釈: docs/ への調査報告のみを置く。
4. 足りなかった情報・検証手段: rmcp 3.9.0 の「MIT のまま」の寄稿がどこかは、LICENSE 本文（移行の説明 + Apache-2.0 + MIT + CC-BY-4.0 の3条文）からは特定できず、寄稿者ごとの再許諸台帳を upstream で照会するしかない。全推移依存の NOTICE/SBOM 作成は U05 のまま今回の対象外。
5. 改善案: docs/agents/issue-tracker.md（または Issue テンプレート）の「Issue 本文の書き方」に「検証系 Issue には成果物の置き場所（crate・テスト・文書のどれに残すか）と想定規模を書く」を足す。

## 実装者メモ（round 2）

1. 詰まった箇所: 手順6（pr-size）: 積み上げ PR（PR 2）の行数を `mise run pr-size`（base=origin/main）で測ると PR 1 の分まで数えてしまうため、`bash scripts/pr-size.sh --base <PR 1 のブランチ>` で測った。ほか: rebase 中に origin/main が進み（pelway-audit-checkpoint 追加）、Cargo.toml のコメントと Cargo.lock が競合した。
2. 曖昧だった文: 「PR 2 は PR 1 のブランチに積んで Draft で出す」（指揮役の t3 thread send）。GitHub の base を PR 1 のブランチにするのか、commit を積むだけで base は main にするのかが書かれていない。
3. 仮定と解釈: PR 2 の GitHub 上の base を PR 1 のブランチにした積み上げ PR として作ると解釈した（docs/worklogs/2026-11-10-platform-pr-ci-r1.md に stacked-PR の CI に関する注記が既にある）。選ばなかった解釈: 両 PR を base=main で独立に出す（PR 2 に PR 1 と同じ差分が二重に載る）。
4. 足りなかった情報・検証手段: 分割後の Cargo.lock の依存の減り方の確認は `cargo tree -p pelway-gateway` で行った（feature を落とすと rmcp の optional 依存が減る）。
5. 改善案: scripts/pr-size.sh の --help の説明文に「積み上げ PR では --base に親ブランチを指定する」を足す。

