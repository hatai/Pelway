# Issue #315 の作業ログ

## 実装者メモ（round 1）

対象 head: 99c5a7240d6a168a27a821d313b986cd9ea02522（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: 環境: `/home` が read-only マウントのため `agent-sandbox.test.sh` の plugin file-tool 検査がセットアップ（`$HOME/.t3/worktrees/sbx-plug-test` と `$HOME/sbx-plug-out` の mkdir）自体で失敗し、`mise run verify` の script-test が赤になった。`~/.cache` も ro で mbx のキャッシュが書けず、`XDG_CACHE_HOME` と `MBX_CACHE_DIR` を `/tmp` 側に向けて verify の残りを通した
2. 曖昧だった文: 「スレッドに送る指示は、Issue 番号か PR 番号と skill 名だけにする。例外はラベル（例: `pr-size-exception`）と Issue のコメントで伝え、指示の文に書き足さない。」（`.agents/skills/command-team/SKILL.md`「決まり（毎回守る）」）
3. 仮定と解釈: スキル改訂の Issue は Issue 本文の「対象スキル・変更の種類」から指揮役が識別できると仮定した（専用ラベルは無い）。指示の 1 行目を「skill implement-issue と skill improve-skills に従う」とする形を選んだ（skill 名だけで構成されるので決まりに収まると解釈）。Issue コメントでの伝達は選ばなかった: 入口B以降を読む指定は improve-skills の本文が担うため、指示への書き足しは「文を書き足さない」に触れ、コメント伝達は指示の実体を GitHub 側に分散させるため
4. 足りなかった情報・検証手段: スキル改訂 Issue を機械的に識別する手段（ラベルや題名の決まり）が無く、指揮役が本文を読んで判断する形になった。script-test の agent-sandbox 検査は `/home` が書き込み可の環境でしか動かず、ローカルの verify は完全には再現できない（AC5 の「必須ジョブが緑」は CI の実行結果に委ねる）
5. 改善案: `.agents/skills/improve-skills/SKILL.md` の「手順 2. 起票して、着手の合図を得る（入口A）」に「タイトルは『スキル改訂: <対象スキル>』の形にする」を足す（command-team が割り当て時に指示を変える判定を機械的にできる）

使ったスキル: implement-issue（`.agents/skills/implement-issue`）
