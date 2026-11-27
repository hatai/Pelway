# Issue #749 の作業ログ

## 実装者メモ（round 1）

対象 head: 8bbf4a1aa402b07b2b5e79156e7662ef4c8bee28

1. 詰まった箇所: 手順 6: 1 回目の `mise run verify` は exit 101 だった。`.agents/skills/verify-pelway/verify-run.test.sh` は、この環境の `/tmp` と `/home` が nobody（uid 65534）所有のため `dir_chain_safe` に落ちた（Issue #480 の作業ログと同じ）。cargo はビルドの途中で `target/` の下の rlib が見つからなくなり、リンクに失敗した（`target` は 01:31 に通常のディレクトリになっていた。2 回目は mbx が「could not replace the managed target directory … the build continues in the existing target directory」を出して通った）。`TMPDIR=/dev/shm/opencode-749`（uid 1000 所有のチェーン。この場所はシェルの呼び出しごとに消えるため、同じ呼び出し内で作った）で再実行すると exit 0（`Finished in 218.58s`、`script-test: 61 tests ok`）。メモのコミット後の `ab07f338f52064e37ecf6ac373617425315cef36` の作業ツリーでも exit 0（`Finished in 182.84s`）。差分は Markdown だけで、失敗した試験は差分を読まない。
   詰まった箇所: 手順 7: PR 本文の一時ファイルを `write` ツールで `/tmp/opencode/issue-749/pr-body.md` に作ろうとしたら、sandbox に拒否された（`pelway-sandbox: write: worktree 外への書き込みは禁止`）。抜け道は使わず、skill `write-pull-request` のとおり `--body-file -` の heredoc で渡した。
2. 曖昧だった文: 「レビューの委託と、修正⇔レビューの繰り返しは、この手順（手順 8〜9）の内側で自分で行う。」（`.agents/skills/implement-issue/SKILL.md` の「決まり」）。開発リードの指示は「レビューは自分で立てず、必要になったら PR 番号と head を指揮役に伝えてください」で、`docs/agents/decisions.md` に該当する決定は無かった。指示どおり委託せず、`waiting-conductor` で終えた。
3. 仮定と解釈: 受け入れ条件の「pr-size の止まり方の項」は、「止まる条件」の表の pr-size の行と手順 6 の pr-size の箇条書きの両方と解した。積み上げ PR の base の決まりは、実装担当が読む手順 8 にも正本（skill `command-team` の「分割を認めるとき」）への参照付きで書いた。「ターンの終わりの状態の行」と「巡回から定型の指示を受けたとき」の表は、Issue の対象外（手順書のほかの節）に当たるとして変えていない。`agent:working` は指揮役の割り当てのものを引き継いだ（`pelway-dev[bot]` 名義の取得コメントは他に無かった）。
4. 足りなかった情報・検証手段: GitHub の自動 retarget は、#744 の timeline の `automatic_base_change_succeeded`（#743 のマージの 3 秒後）で確かめた。必須チェックが base が main の PR だけで走ることは、`.github/workflows/verify.yml` の `pull_request.branches: [main]` と、`pr-size` が同じ workflow のジョブであることで確かめた。`/tmp` の所有は `stat -c '%u' /tmp` の出力（65534）で確かめた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「ターンの終わりの状態の行」に「`waiting-conductor` には、手順 8 で待っているものがそろうのを待つ場合（分割 PR の `<!-- split-plan -->` 待ち、積み上げ PR の親 PR のマージ待ち）を含む」を足す。`docs/agents/grokbot-runbook.md` の「毎回の巡回でスレッドを点検する」の `state: waiting-conductor` の行に「base が main でない PR にはレビュースレッドを立てない」を足す。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`
