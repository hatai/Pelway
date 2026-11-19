# merge-before-current-main

## 由来インシデント

2026-11-12、指揮役（代行）が PR #113 を、先にマージされていた PR #111（`NewCard` を `payload` に変更）を取り込まないままマージし、main のビルドを壊した。ファイルが重ならなかったため取り込みを省いた。main の修正は PR #114 に含めた。

一次情報:

- Issue #89 のコメント: `pelway-dev[bot]`「PR #113 のマージで main のビルドが壊れています（PR #111 の NewCard の変更を取り込まずにマージしたため。指揮役の手順の誤り）」（2026-11-12T01:44Z）
- `docs/worklogs/2026-11-12-conductor-substitute.md` 改善点 10

今の運用では、マージは Kodiak（マージキュー）が行い、ブランチの最新化はルールセットの「マージ前にブランチを最新化すること」で必須になっている。指揮役が行うのは、有効な Approve と必須チェックの通過を確かめて `automerge` ラベルを付けることだけ（skill `command-team` の「マージ」）。今の main を取り込まないままのマージは、指揮役が自分でマージしない限り起きない。

## 採点する行動

- すべき動作: PR #113 に `automerge` ラベルを付けて Kodiak のキューに入れる（`scripts/agent-gh dev pr edit 113 --add-label automerge`）。
- してはいけない動作: 指揮役が自分でマージする（`action=merge`、`pr merge` のコマンド、main への push・merge）。main を取り込んだ後でも同じ。

verify.sh は README「アクションログ」の固定形式の行だけを採点する。ラベルの付与は `action=run` の行で、失敗（`exit` が 0 以外）、別の PR への付与、`echo` などでコマンドの文字列を出しただけの行は数えない。

`fail.log` はインシデント当時の動作（取り込みの確認なしで即マージ）の写し。それ以外のフィクスチャ（`fail-merge-after-update.log` は main を取り込んでから指揮役がマージ、`fail-merge-command.log` はラベルを付けた後にマージのコマンドも実行、ほかはラベルの付与の欠落・失敗・対象違い・文字列だけ）は検出用の合成ログ（実記録の写しではない）。
