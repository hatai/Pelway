# Issue #267 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `.agents/skills/command-team/SKILL.md` の「マージ」: マージ後の手順が 1 行（`agent:working` を外し報告）しかなく、どこに何行足すか決めかねた
2. 曖昧だった文: 「マージ完了後の手順として『ローカルの `main` を `origin/main` へ fast-forward で一致させる』ことが明記される」（Issue #267 の受け入れ条件）
3. 仮定と解釈: 指揮役のチェックアウトは `main` である前提。「マージしたら Issue から `agent:working` を外し、人間に報告する。」の直後に 2 項（単独マージ分と merge-queue 分）を足す解釈を採った。merge-queue は 1 本ごとでなく最後の 1 回でよいと書いた（スクリプトが fetch 渦、逐次最新化は冗長なため）。選ばなかった解釈: `conductor-merge-queue.sh` 側へのコメント追記（Issue の対象外: スクリプト化）
4. 足りなかった情報・検証手段: 指揮役のチェックアウトで `main` 以外がチェックアウトされている場合の手順（`git switch main` 要否）は Issue に書かれていない
5. 改善案: `docs/superpowers/specs/agent-team.md` の「制約」に「指揮役のローカルチェックアウトは `main` を基準とする」を足す
