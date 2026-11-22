# Issue #595 の作業ログ

## 実装者メモ（round 1）

対象 head: c25cb12a90be62ac7bd84730e47a89f5d4468012

1. 詰まった箇所: 手順 6: 新規テスト `scripts/tests/pr-body-round-history-docs.test.sh` を mode 100755 で push したため `agent-gh-resign` が署名を付けなかった。script-test は `bash "$t"` で実行するので mode 100644 に直して amend・force-with-lease で作り直し、署名付き（verified: true）にした。
2. 曖昧だった文: 「write-pull-request が上流から取り込んだスキルで直せないなら、implement-issue の側に優先を書く」（Issue #595「受け入れ条件」）
3. 仮定と解釈: `docs/agents/external-skills.md` に write-pull-request が無いため、ローカル管理のスキルとみなし、write-pull-request 側を直した。implement-issue 側への優先の記述は足していない。手順 4 の red の出力は round 1 の表に貼り、修正の round で置き換わるものと解釈した。
4. 足りなかった情報・検証手段: スキルが上流由来かを一目で判別する印（frontmatter など）が SKILL.md に無く、`docs/agents/external-skills.md` を grep して確かめた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「手順 4」に「新規のテストファイルは mode 100644 で作る（script-test は bash で実行し、100755 の新規ファイルは agent-gh-resign で署名されない）」と足す。

使ったスキル: implement-issue, write-pull-request
