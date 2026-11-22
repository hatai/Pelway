# Issue #621 の作業ログ

## 実装者メモ（round 1）

対象 head: b0c720e650564e78e74659292bf0efc99f722183

1. 詰まった箇所: `scripts/demo-check/lib.sh`: shebang を持たず、Issue の数え方（1 行目の `#!`）では拾われないが、旧タスクの `shellcheck scripts/demo-check/*.sh` は検査していた。shebang だけで集めると検査が弱まるところだった。
2. 曖昧だった文: 「`scripts/demo-check/*.sh` は既定の severity、それ以外は `--severity=warning`」（Issue #621「受け入れ条件」）
3. 仮定と解釈: demo-check は glob 全体（shebang の無い lib.sh を含む 7 本）を既定の severity で検査し、出力の「N files checked」は shebang 一致の数（find と一致させる数）に限って lib.sh は別に数えると解釈した。shebang 一致分だけを検査する解釈は、#265 からの検査を弱めるため選ばなかった。数は Issue 起票時の 65 本から、足した `scripts/shellcheck.sh` の分だけ増えて 66 本。
4. 足りなかった情報・検証手段: 試験は `mise run shellcheck` を一時ディレクトリの写しで動かす手段が無い（mise.toml の trust とツールの解決が要る）ため、タスクの本体を `scripts/shellcheck.sh [<root>]` に出し、写しのルートに当てて確かめた。info 級の例に挙げた SC2086（`x=1; echo $x`）は 0.11.0 では出ないため、SC2016 に替えた。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「検査範囲を数え方で定める条件では、今の範囲に数え方から外れるファイル（shebang の無い source 用ファイルなど）が無いかを起票時に確かめる」を足す。

使ったスキル: なし
