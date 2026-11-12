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
