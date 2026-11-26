# Issue #805 の作業ログ

## 実装者メモ（round 1）

対象 head: b9146d877131afd8a29f4c184a12edced9587591

1. 詰まった箇所: 手順 6: verify ログを無視されない `verify.log` に書いて hk の trailing_whitespace に落ち、`mutants.out/` へ変えた。ホストの `/tmp` が nobody 所有で verify-pelway が落ちるため TMPDIR=/dev/shm にし、shellcheck.test.sh の並行実行だけが Terminated で落ちた（単独では pass）。CI では main の `agent-team.toml` 更新にテストのピンが追いつかず、その直後の main 修正にもカンマ誤りがあったため、origin/main を取り込んで `scripts/tests/agent-team-config.test.sh` の区切りを直した。
2. 曖昧だった文: 「今ある決定のうち、手順書に反映されていないもの（少なくとも t629u と #725）が載っている」（Issue #805 の「受け入れ条件」）
3. 仮定と解釈: 載せる範囲は、手順書と食い違う・食い違いうる運用の決定に限った。`#725` は独立した決定ではなく t629u の反映 Issue とし、#806・#807・#811 を載せた。道具追加の #809・#810 や整理の #813 は現行の手順と食い違わないため載せなかった。選ばなかった解釈: t629u だけを載せる。
4. 足りなかった情報・検証手段: t629u 以外の決定 ID の採番規則と、hatai が確かめた現在の決定一覧。Issue・PR・作業ログから拾う範囲では、ID の無い決定は Issue 番号を使うほかなかった。
5. 改善案: `docs/agents/decisions.md` の「運用の決定」に「ID は既存の識別子を使い、無ければ反映する Issue 番号から作る」を足した（今回実施済み）。

使ったスキル: implement-issue、writing-for-agents、write-pull-request

## 実装者メモ（round 2）

対象 head: c8ce01b88d714e8108b4b4aad6a2cf01e4f52101

1. 詰まった箇所: `.agents/skills/implement-issue/SKILL.md` の決まり: codex の must-fix は、決定を優先する範囲と参照する版の両方が未定義だった。main 取り込みでは main 側が `agent-team-config.test.sh` の同じ区切り誤りを先に直していたため、取り込み後の木は変わらず、重複したテスト修正は diff から外れた。round 2 の verify は `conductor-watch-decision.test.sh` が並行実行時だけ落ち、単独実行は pass、再実行した全体 verify は pass だった。
2. 曖昧だった文: 「該当する決定があるときは、その決定を手順書と同じ根拠として扱う。」（`.agents/skills/implement-issue/SKILL.md` の「決まり」）
3. 仮定と解釈: 「個別承認」は force push・マージ・`.github/workflows/` の push・pr-size の例外など hatai が個別に判断する既存の承認と解釈した。decisions.md は手順書全体に優先するのではなく、直接食い違う運用手順だけに優先すると読み、main の承認済みの版を参照する形にした。
4. 足りなかった情報・検証手段: 「個別承認」の語の定義は手順書に無く、t629u の決定文と既存の例外の扱いから意味を補った。文言の意味の判定はレビュー担当に委ねるため、該当文は PR 本文に引用し直す。
5. 改善案: 今回の指摘で両スキルの文を、main の承認済みの版を読むこと・既存の確認と承認を適用すること・直接食い違う手順に限って優先することの 3 点が分かる形に直した（今回実施済み）。

使ったスキル: implement-issue、receiving-code-review、writing-for-agents、diagnosing-bugs、write-pull-request
