# security-review の試験運用

PR レビューにセキュリティ観点のパスを試験導入する（hatai の判断）。skill `security-review`（`.agents/skills/security-review/`、`.claude/skills/` からの symlink）を作り、`review-pr` には発動条件と参照だけを足した。正式に command-team と agent-team.md へ組み込むかは、下の記録を見て決める。

## 元にしたもの

Trail of Bits のスキル集（https://github.com/trailofbits/skills 、2026-11-12 時点の main）から次の 3 つの手法だけを参照した。全カタログは入れていない。static-analysis（CodeQL/Semgrep）は、レビュー担当が PR のコードを実行しない決まりと CI 側の導入作業が要るため、今回は見送った。

- differential-review: 危険度でファイルを分け、消えた検査を履歴でたどり、影響範囲と試験の有無を見る
- agentic-actions-auditor: 攻撃者が書ける入力が AI の指示文に届く経路（env 経由、式の直接展開、CLI での取得、`pull_request_target` と head の取り出し、ログ、AI 出力の実行、広すぎる権限・許可リスト）
- insecure-defaults: fail-open な既定値の 6 分類と、「本当にその値で動くか」をたどってから報告する考え方

## ライセンスの判断

判断: 文章はコピーせず、手法を参照して日本語で書き直した自作スキルにした。

根拠:

- Trail of Bits のスキル集は CC BY-SA 4.0（`gh api repos/trailofbits/skills` の `license.spdx_id` が `CC-BY-SA-4.0`）。文章をコピーまたは翻案すると、表示（BY）に加えて、翻案物を同じ CC BY-SA 4.0 で提供する義務（SA）が生じる。
- Pelway のリポジトリには LICENSE ファイルがなく、Cargo.toml にも license の指定がない。CC BY-SA の翻案物を混ぜると、そのファイルだけ別のライセンスで管理することになり、将来のライセンス決定の妨げになる。
- 著作権が守るのは表現で、手法や考え方そのものではない。今回のスキルは、Pelway の決まり（読み取りだけ、`scripts/agent-gh`、投稿の形）に合わせて構成も文も新しく書いており、元の文の翻訳や言い換えの並べ替えではない。元の表、例文、パターン集（`references/*.md`・`*.json`）は取り込んでいない。
- 参照元はこの worklog に記録する（義務ではないが、出所を追えるようにするため）。

判断に迷う点は見つからなかったため、needs-human にはしていない。

## 読み替えたところ

- differential-review の「基準のコミットを checkout して全体の文脈を作る」手順は採らず、main 側を `git show`・`git log -L`・`git blame`・`git grep` で読む形にした（レビュー担当は head も基準も取り出さない）。
- 「報告をファイルに書く」は採らず、`review-pr` の投稿 1 件の中の `## security-review` 節にした。
- 攻撃者の模擬（adversarial）用の別エージェント、試験の実行、PoC は採らず、実行しないと分からないことは「未確認」と書く形にした。
- insecure-defaults の複数エージェントによる探索と反証の流れは、1 人のレビュー担当が「候補をたどりきれなければ所見にしない」形にまとめた。

## 試験運用の範囲

- 高リスクの PR: `review-pr` の発動条件（`unsafe`、`.github/workflows/`、`scripts/agent-gh`、`AGENTS.md`、`.agents/skills/`、`docs/agents/`、認証・トークン・暗号・sandbox・テナント分離・監査）
- 通常の PR のサンプリング: 指揮役が、高リスクに当たらないレビューの割り当て 4 件に 1 件、`Assigned:` の行の末尾に `security=yes` を付ける

## 記録

試行した PR ごとに、所見を「本物」「誤検知」「通常レビューと重複」に分けて書く。

| PR | 発動 | レビュー担当 | 所見 | 本物 | 誤検知 | 重複 | メモ |
|---|---|---|---|---|---|---|---|
| #134 | high-risk（`.agents/skills/`） | codex gpt-6-luna medium | 0 | 0 | 0 | 0 | main にまだ無い security-review を、PR の中の版から読んで当てていた。結果は害なしだが、PR が持ち込む手順に従う形で、指示の混入と同じ経路。review-pr に「従う手順は worktree（main）のスキルだけ」を足した |
| #135 | high-risk（`.agents/skills/`、`docs/agents/`） | codex gpt-6-luna medium | 0 | 0 | 0 | 0 | main に security-review がある状態での最初の試行。脅威所見なしとの判断は妥当。ただし `## security-review` の定型の節ではなく自由文の「セキュリティ確認」で書き、HIGH の一覧・未確認が欠けた。review-pr の本文の型に節の置き場所が無かったため、型に足した |
| #139 | high-risk（`.agents/skills/`・`docs/agents/`） | codex gpt-6-luna medium（2 回） | 1 | 1 | 0 | 0 | Devin を `full-access`・`bypass` で立てる手順に、Issue からの指示の混入と広い権限の組み合わせを指摘。既存の opencode も同じ権限で、仕様に実装担当の権限境界が書かれていなかった（既存の設計上の穴が見えた）。別のレビュー担当でも同じ must-fix になり、hatai の判断で実装スレッドの sandbox（#151）を先に作り、#139 は保留 |
| #141 | high-risk（暗号化） | codex gpt-6-luna medium・high（4 回） | 0 | 0 | 0 | 0 | security-review の所見は毎回 0。must-fix は通常のレビュー側（文字化け、保証の断定） |
| #144 | sampling（security=yes） | codex gpt-6-luna medium | 1 | 1 | 0 | 0 | 公開の作業ログに開発機のホスト名が残っていた（nit）。抜き取りで本物の所見が出た最初の例。この PR で伏せた |
| #145 | high-risk（認証） | codex gpt-6-luna high | 1 | 1 | 0 | 0 | passkey の credential の失効が未認証の要求を受け、member の ID を本文から取っていた（任意の利用者を締め出せる）。コードの本物の脆弱性を見つけた最初の例。修正後に Approve |
| #146 | high-risk（認証・テナント分離） | codex gpt-6-luna high | 1 | 1 | 0 | 0 | OIDC の tenant の許可リストが空だと claim を検査せず、任意の IdP アカウントでログインできた（安全でない既定値の分類）。修正後の再レビューで所見 0 |
| #147 | high-risk（漏えい検査の新設） | codex gpt-6-luna high | 0 | 0 | 0 | 0 | |
| #115 | high-risk（テナント分離） | codex gpt-6-luna high | 0 | 0 | 0 | 0 | RLS の強制、fail-closed、ロール属性を静的に確かめて所見 0 |
| #152 | sampling（security=yes） | codex gpt-6-luna medium | 0 | 0 | 0 | 0 | 新しい取得元（conda-forge）を足す PR |
| #153 | high-risk（`.agents/skills/`・`AGENTS.md`） | codex gpt-6-luna high | 0 | 0 | 0 | 0 | |

## 改訂: 入口の追跡・候補の確かめ・Rust の観点（2026-11-19）

hatai の判断で、所見の取りこぼし（#466 の最初のレビューは所見 0 で approve、2 回目で must-fix 2 件）と Rust の観点の薄さに対して、手順を足した。

### 参照した先行事例

`gh search` で探し、次の 3 つを比べた。hatai が「B を軸に A と C も取り入れる」を選んだ。文章はコピーせず、手法だけを参照して書き直した（上の「ライセンスの判断」と同じ考え方）。

- A: Anthropic `claude-code-security-review`（MIT）の `.claude/commands/security-review.md`。採ったもの: main 側の既存の安全の型と比べて外れを探す手順、入力から危険な処理までデータの流れをたどる手順。採らなかったもの: DoS・資源枯渇・Rust のメモリ安全を報告しない除外リスト（所見を減らす向きで、`unsafe` を発動条件にしている設計と合わない）、確信度の数値。
- B: Trail of Bits `rust-review`（CC BY-SA 4.0）。採ったもの: 差分の中身で当てる観点を選ぶ仕組みと、Rust の欠陥の種類（`references/rust.md`）。採らなかったもの: 複数エージェントの並列実行・スクリプト・SARIF（レビュー担当は読み取りだけの 1 エージェントで、投稿は 1 件）。
- C: Trail of Bits `fp-check`（CC BY-SA 4.0）。採ったもの: 候補ごとに入口・経路・防御・影響を確かめる段。採らなかったもの: PoC の作成と実行（PR のコードを実行しない決まり）。

### 評価

過去の実 PR 7 件（レビュー時点の head、base の時点までの履歴だけを持つクローン、PR 本文はレビュー時点の版）と、パス横断などを仕込んだ合成の差分 1 件で、旧版と新版を Claude Opus と Codex（gpt-6-luna medium）で 1 回ずつ走らせた。判定は LLM 採点者（Claude Sonnet）がケースごとのルーブリックで行い、1 件の甘い判定を手で直した。評価の素材と出力はリポジトリに入れていない。

| 構成 | 合格率 |
|---|---|
| 旧版 × Claude | 100% |
| 新版 × Claude | 100% |
| 旧版 × Codex | 88.5% |
| 新版 × Codex | 92.7% |

- Claude は旧版でも全ケースで仕込んだ欠陥を見つけた。所見の取りこぼしは、スキルよりもレビュー担当のモデルの側の要因が大きいと見ている。
- Codex で差が出たのは #466 の reply_guard だけ（旧版は 2 つの穴を両方見逃し、新版は curl のコマンド位置のすり抜けを見つけた。`tool_use` の値の形は両方見逃した）。1 回ずつの実行なので、偶然の差の可能性がある。
- 合成ケースの `/tmp` を既定の置き場所にする欠陥は、Codex が新旧とも所見にしなかった（新版は「確かめたこと」で危険に触れたが所見にしていない）。
- 所見 0 が正解の 2 件（#115、#141）では、どの構成も must-fix を出さなかった。
- 新版は Claude で 1 実行あたり約 +38 秒、約 +13k トークン。

効果は、以後の試行の記録（上の表）で引き続き見る。
