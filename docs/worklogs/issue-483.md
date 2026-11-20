# Issue #483 の作業ログ

## 実装者メモ（round 1）

対象 head: 6ab35f3ffa69fdfad132129703d4fee1487f2ce6

1. 詰まった箇所: scripts/agent-gh: qa 用の秘密の名前（`PELWAY_QA_INSTALLATION_ID`・`PELWAY_QA_PRIVATE_KEY`）が既存 `app_token` の命名規則（`<PREFIX>_APP_INSTALLATION_ID`・`<PREFIX>_APP_KEY`）に合わず、`app_token` を接頭辞ではなく 3 つの名前を受け取る形に変えた。docs/dev-environment.md: 「Approve・マージ・ラベル操作などは権限がなく 403 になる」と一度書いたが、Issues: Read and write はラベル操作を権限上は許しうるため、「権限上は通りうる操作も許可リスト側が拒否する」旨に書き直した。
2. 曖昧だった文: 「設定は `PELWAY_AGENT_FNOX_CONFIG` で `~/.config/pelway-agent-qa/fnox.toml` を指す。既定値は使わない。設定がなければ止まる。」（Issue #483 本文「やること」）
3. 仮定と解釈: 「既定値は使わない」は、未設定時に dev/review 用の `~/.config/pelway-agent/fnox.toml` を読まないことと解釈した（設定値が qa 用のパスと一致するかの検査までは求められていないと読み、パス文字列の一致検査は脆いため未設定の拒否だけにした）。拒否の終了コードは既存体系に合わせて 2（役割・引数の誤り）に統一し、`PELWAY_AGENT_FNOX_CONFIG` 未設定は 3（トークンを取れない）に振った（受け入れ条件は「0 以外」のみ）。コメント送信は `gh api` ではなく `gh issue comment <番号> --repo hatai/Pelway --body-file` を選んだ（内部で `POST /repos/.../comments` のみを呼び、JSON 組み立てが不要で `--body-file -` の標準入力を gh が処理する）。
4. 足りなかった情報・検証手段: pelway-qa の実トークンでの動作（コメント作者が `pelway-qa[bot]` になるか、Approve/マージが 403 か）は秘密を読めないこの環境では確かめられず、Issue の「実測」節のとおりマージ後に回した。テストは偽の fnox/gh/git で、拒否が fnox・gh・git の呼び出しより前であることを終了コードと呼び出しログの不在で確かめる形にした。
5. 改善案: `docs/superpowers/specs/agent-team.md` の「役割と GitHub 上の名義」に「QA 専用の第 3 の App `pelway-qa` があり、`scripts/agent-gh qa comment` で Issue/PR へのコメントのみを書ける」の一文（同節の「App は 2 つに分ける」と役割表は現状と食い違うが、QA の運用手順の本格整備は #480 の範囲のためここでは触らない）。

使ったスキル: `.agents/skills/implement-issue`
