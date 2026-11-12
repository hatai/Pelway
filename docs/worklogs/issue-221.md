# Issue #221 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: scripts/agent-gh dev: 原型の rerun-build/rerun の `.claude/skills/investigate-bug/SKILL.md` を読むのに素の `gh` が使えなかったため、`scripts/agent-gh dev api /repos/rerun-build/rerun/contents/...` で取得した
2. 曖昧だった文: 「未決事項: 調査メモの置き場所（docs/worklogs 配下か、別ディレクトリか）」（Issue #221 本文「未決事項」）
3. 仮定と解釈: 調査メモは `docs/worklogs/bugs/<slug>/report.md` に置くとした。AGENTS.md の「Worklogs live under docs/worklogs/」の流儀に乗り、調査は Issue 番号を持たないため `issue-<N>.md` 形式に合わないこと、ディレクトリにすれば rerun 原型の `.context/bugs/<slug>/` と同じく証拠（ログ断片・キャプチャ）を同梱できることによる。選ばなかった解釈: `docs/worklogs/` 直下のフラットファイル（日付プレフィックス形式）— 証拠ファイルの置き場所がなくなるため
4. 足りなかった情報・検証手段: Pelway にまだ本番・共有環境の実在名（環境名、ログ基盤、監視）が無いため、スキルには具体名を書けず一般論（本番・ステージングのサーバーと DB 等）で書いた。実環境が決まったら具体化する必要がある
5. 改善案: docs/superpowers/specs/agent-team.md の「タスクの流れ」に「バグの報告は investigate-bug で調査してから write-issue で起票する」を足す（Issue #221 はスキル本文のみを対象としているため本 PR では変更していない）
