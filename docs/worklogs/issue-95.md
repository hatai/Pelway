# Issue #95 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 手順6（mise run pr-size）: 実装一式（`crates/pelway-llm` の src 758 行 + モックHTTPサーバー結合試験 706 行、Cargo.lock・mise.lock は linguist-generated で除外）で追加 +1464 行となり上限 800 を超えた。Issue の分割案 1)（ツールチェーン更新、+20 行弱）だけをこの round の Draft PR とし、2)・3) は Issue への分割案コメントに回した。ほか: reqwest 0.13 の TLS feature 名は `rustls`（`rustls-tls` ではない）、rig 0.46 は chat の system メッセージ本文を `[{"type":"text","text":...}]` の配列に直列化する（`json_object` 向けスキーマ注入の試験のアサーションを合わせた）。
2. 曖昧だった文: 「800行を超えそうなら、1) ツールチェーンの更新、2) trait と rig-core 接続と試験、3) 測定コマンドと合成データ、に分ける（split-plan は指揮役が書く）。」（Issue #95 の「規模の見積もり」）— 2) の trait と試験だけで既に 800 行を超える見込みのとき、実装者が 2) をさらに分割してよいかが決まっていない。
3. 仮定と解釈: 仮定: 分割単位の決定は指揮役の権限（skill `implement-issue` の停止条件と同じ）。解釈: 上限内に収まる 1) だけを Draft PR として先に進め、2)・3) の分割案は Issue コメントに書いて指揮役の split-plan を待つ。選ばなかった: 試験や説明を削って 800 行に収める（skill が禁止）、行数を無視して全量を 1 PR にする（PR の上限ルール違反）。
4. 足りなかった情報・検証手段: rig-core 0.46 の呼び出し経路（`OpenAIConfig::connect(DynHttpClient)` → `Model<Chat>::call`、OPENAI 弁話の `completion()` は Responses API に行くため `.chat()` を使う点）— docs/research/rust-llm-client-comparison.md の記述と一致したが、~/.cargo/registry のソースで確認した。2) を分割する場合の試験の切り出し単位。
5. 改善案: docs/research/rust-llm-client-comparison.md の「rig-core」の節に「0.46 の OPENAI 弁話は `completion()` が Responses API に行く。Chat Completions 限定なら `chat()` を使う」を追記する。

## 実装者メモ（round 2）

1. 詰まった箇所: 手順4（clippy）: 1.99.1 の `clippy::collapsible_if`（schema.rs 5箇所・gateway.rs 1箇所）、`clippy::expect_used` 系の `is_some` 後 `expect`（gateway.rs）、`clippy::await_holding_lock`（環境変数の排他ロックを await にまたがせたテスト）を出した。let-chain（edition 2024）で畳み、`expect` は `if let ... &&` で解消し、ロックは「同じロックを取るタスクが無い」ことをコメント付きで `#[allow]` した。ほか: 手順6（pr-size）: PR 2 の差分は `--base agent/95-toolchain-1-99-1` で測って +1462 となり上限 800 を超えるが、指揮役の split-plan コメント（hatai 作成）が PR 2 を「新しい crate とモックの試験」一式に定めており、chat での指示も同じため、例外ラベル（`pr-size-exception`）の付与を PR 本文で依頼してこのまま出す。
2. 曖昧だった文: 「後の PR は前の PR のブランチに積み、前の PR のマージの前に base を main に替える。」（Issue #95 の split-plan コメント）— 「base を main に替える」主体とタイミング（マージ後に GitHub が自動で替えるのを待つか、実装者が先に rebase するか）が決まっていない。
3. 仮定と解釈: 仮定: マージ前の PR 2 は base=PR 1 ブランチの積み上げ PR とする（issue #96 round 2 と同じ）。解釈: PR #156 マージ後に、実装者が PR 2 のブランチへ main を取り込んで（merge または rebase）base を main に替える。選ばなかった: PR #156 のマージを待ってから単独 PR として出す（レビュー期間が直列になり、split-plan の「積む」に反する）。
4. 足りなかった情報・検証手段: pr-size 上限超過の PR を出すときの例外ラベル付与の-flow（AGENTS.md はラベルか指揮役のマーカーコメントを求めるが、chat 指示との優先順位）。CI が pr-size を fail させるなら PR 2 の CI が赤くなる可能性（.github/workflows で pr-size を走らせているか未確認）。
5. 改善案: `.github/pull_request_template.md` の「レビューと QA」に「pr-size 上限を超える PR は、指揮役の split-plan・例外ラベルの根拠をここに引用する」を足す。

## 実装者メモ（round 3）

1. 詰まった箇所: crates/dev-llm-eval/tests/eval.rs: モックが課題を応答本文の語で判別する設計にしたところ、handover 課題のスキーマ（プロパティは `items` のみ）には「handover」の語が現れず、判別に失敗して schema_ok=0 になった。スキーマのプロパティ名（`decisions`/`risk`/`items`）で判別し直した。ほか: `#[tokio::main]` は既定で `rt-multi-thread` feature を要求するため、pelway-gateway と同じ `flavor = "current_thread"` 指定にした。
2. 曖昧だった文: 「測定用のコマンド（例 `mise run llm-eval`）を作る。」（Issue #95 の受け入れ条件）— コマンドを `crates/pelway-llm` のバイナリにするか、開発用 crate（`pelway-dev-*`）にするかが書かれていない。
3. 仮定と解釈: 仮定: 測定コマンドは開発用ツールで、`mise run verify`（cargo test）の対象に含まれる。解釈: `crates/dev-llm-eval`（パッケージ名 `pelway-dev-llm-eval`、dev-smoke と同じ命名）に `src/lib.rs`（測定ロジック）+ `src/main.rs`（CLI）+ `data/eval-data.json`（include_str! で埋め込み）を置き、`mise run llm-eval` から `cargo run -p` で呼ぶ。選ばなかった: pelway-llm にバイナリを置く形（tokio が lib の依存に入り、テスト専用の依存が製品側に漏れる）。
4. 足りなかった情報・検証手段: 表の列構成（Issue は「スキーマに合う割合・応答時間・トークン数・検索の当たり（recall@k）」としか書かない）— lib.rs の `markdown` で列を固定したが、`docs/research/` に置く表の既存フォーマットは無かった。実APIでの出力例: なし（hatai の実行待ち）。
5. 改善案: docs/research/README.md に「測定結果の表は `mise run llm-eval` の出力をそのまま貼る」を追記する。



## 実装者メモ（round 4）

1. 詰まった箇所: 手順3（PR #163 の作り直し）: ブランチを main に戻して `gateway.rs` と結合試験を外した結果、`lib.rs` の再エクスポートと `Cargo.toml`（`rig-reqwest`・`http`・`tokio` は PR 3 で追加し直す）の切り分けが必要になった。ほか: tests/pure.rs で `ProviderResponseError` は `rig_core::error` ではなく root の再エクスポート（`rig_core::ProviderResponseError`）でなければならず、`ProviderError::Url` の構築には url crate（dev 依存に追加）が要った。
2. 曖昧だった文: 「`Debug` にトークンが出ない → それぞれ `cargo test -p pelway-llm` の単体試験」（Issue #95 の split-plan コメント、PR 2 の行）— この時点の crate にはトークンを保持する型が無く（gateway は PR 3）、「Debug に出ない」を何の型について確かめるかが決まっていない。
3. 仮定と解釈: 仮定: トークンの「送信前の型」も PR 2 の `token` の範囲に含まれる。解釈: `TokenSource::token` の返り型を `String` から `SecretToken`（`Debug` は `[redacted]`、取り出しは `expose` だけ）に変え、単体試験で包んでも Debug に出ないことを確かめる。選ばなかった: gateway（PR 3）までこの条件の確認を先送りする（split-plan が PR 2 の受け入れ条件に挙げているため）。
4. 足りなかった情報・検証手段: `reqwest::Error` には公開コンストラクタが無く、単体試験で未送信（is_connect）/結果不明の実経路を再現できないため、それらは PR 4 の結合試験（接続拒否・タイムアウト）で確かめる。エラー分類の試験が分類の実装に結び付いていることの確認は変異検査（`Json` を Invalid から外すと `error_mapping_invalid_for_parse_and_shape_failures` が FAILED、戻すと 7/7 合格）で行った。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順6に「積み上げ PR の pr-size は `bash scripts/pr-size.sh --base <親ブランチ>` で測る」を明記する（issue #96 round 2 のメモと同じ指摘だが skill 本文に反映されていない）。

## 実装者メモ（round 5）

1. 詰まった箇所: crates/pelway-llm/tests/gateway.rs: 全体から PR 3 の範囲（基本経路と利用量の試験）を切り出す際、python での行範囲削除が `Reply::status` と `await_records` の閉じ括弧と本体を残し、構文エラー（`unexpected closing delimiter`）になった。手直しして `cargo fmt`・clippy で確認した。ほか: `SecretToken` 化に伴い `BearerHook` の `format!("Bearer {token}")` は Display を要求するため `token.expose()` に変えた。
2. 曖昧だった文: 「PR 3『gateway 本体と基本経路の結合試験』| ...（rig-core 接続、HTTP クライアントの差込み、要求ごとのトークン、利用量と上限）、モック HTTP サーバー、chat（スキーマなし／あり）・`json_object`・embed の結合試験」（Issue #95 の split-plan コメント）— 「要求ごとのトークン」の試験（取り出し・秘匿）は PR 4 に置いたまま、PR 3 の範囲でどこまでトークンを確認するかが決まっていない。
3. 仮定と解釈: 仮定: PR 3 には BearerHook の存在確認（1 要求に Bearer が付く）だけで足りる。解釈: 基本経路の chat 試験に `Authorization: Bearer fixed-token` の確認を1行足した。差し替え・秘匿・ログ不出の試験は PR 4 のまま（split-plan の PR 4 行が「トークンの要求ごとの取り出しと秘匿」に置いているため）。選ばなかった: トークン試験一式を PR 3 へ移す（PR 3 が 800 行を超える）。
4. 足りなかった情報・検証手段: PR 3 の試験は round 2 で TDD により開発したもの（当時の RED: `not implemented` で 14 失敗）の再配置であり、PR 3 単独での新規 RED は取れない。配置替え後の合格（5 試験）と `mise run verify` で代用した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「よくある失敗」に「分割で試験を移動するときは移動先 PR で実行結果を引用し、出所（元の round）を明記する」を足す。

## 実装者メモ（round 6）

1. 詰まった箇所: crates/pelway-llm/tests/gateway.rs: PR 3 から切り出した残りの試験を戻す際、full からの切り出しに利用量の試験が重複して入り（E0428）、`Reply::DelayedJson` の列挙子追加と `AtomicUsize` の import 戻し、`TokenSource` の返り型変更（`String` → `SecretToken`）に伴う試験内実装の修正が必要だった。
2. 曖昧だった文: 「PR 4『異常系の結合試験と測定コマンド』| 5xx・タイムアウトで再送しない、接続拒否、不正応答、トークンの要求ごとの取り出しと秘匿、プロキシの結合試験、測定コマンドと合成データ」（Issue #95 の split-plan コメント）— この範囲の実測行数（異常系試験 約 +330、測定コマンド +715 = 約 +1045）が 800 行に収まらないときの扱いが決まっていない。
3. 仮定と解釈: 仮定: 800 行上限は PR ごとに守る（round 1 と同じ。split-plan は例外ラベルには触れない）。解釈: PR 4 は「異常系の結合試験」に絞って出し、測定コマンドと合成データ（実装済み・round 3 で GREEN）は PR 5 としての分割を Issue コメントで提案する。選ばなかった: 2つを1本の PR にする（上限超過）ことと、テストや合成データを削って収める（skill が禁止）こと。
4. 足りなかった情報・検証手段: split-plan 更新時に実装側の実測行数（round 1 コメントの +1464 の内訳と round 3 の +715）が参照されなかったため、PR 4 の見込みが立たなかった。
5. 改善案: `.github/ISSUE_TEMPLATE/agent-task.yml` の「規模の見積もり」の description に「分割案には各 PR の実測・見込み行数を書く」を足す。

## 実装者メモ（round 7）

1. 詰まった箇所: crates/dev-llm-eval/tests/eval.rs: `TokenSource` の返り型を `SecretToken` に変えた（round 4）影響で、試験内の `Fixed` 実装が `Result<String, LlmError>` のまま E0053 になった。`SecretToken::new` で包み直した。ほか: mise.toml への `[tasks.llm-eval]` 追加位置（[tasks.fetch] の直後）に置き直した。
2. 曖昧だった文: 「測定コマンドがモックで動き、`docs/research/` 向けの表を出力する → CI の試験」（Issue #95 の split-plan コメント、PR 4 の行）— 「CI の試験」が `mise run verify`（cargo test）に含まれるモック試験を指すのか、GitHub Actions の workflow を変えることまで含むかが決まっていない。
3. 仮定と解釈: 仮定: CI は既存の `mise run verify` ジョブ（docs/dev-environment.md「CI（GitHub Actions）」）のまま。解釈: 測定コマンドのモック試験（tests/eval.rs）を cargo test に含めることで「CI では外部へつながらず、モックで動くことだけを試す」を満たす。`.github/workflows/` の変更はしない（pelway-dev[bot] には workflow を push する権限がなく、必要なら停止条件に当たる）。選ばなかった: 専用の workflow 追加（権限がなく停止条件に該当するため）。
4. 足りなかった情報・検証手段: PR 5（測定コマンド）の割り当て（指揮役の判断待ち。ブランチ `agent/95-llm-eval-cmd` に実装済み・`mise run verify` EXIT=0・`cargo test -p pelway-dev-llm-eval` 3/3 合格）。
5. 改善案: docs/dev-environment.md の「CI（GitHub Actions）」に「新しい cargo パッケージの試験は cargo test --workspace で自動的に CI に入る（workflow の変更は不要）」を明記する。
