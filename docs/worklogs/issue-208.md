# Issue #208 の作業ログ

## 実装者メモ（round 1）

対象 head: 3498ffbbd61eb9028ae054117ada9c4e9efa2a48

1. 詰まった箇所: `scripts/agent-gh`: この環境は直接の DNS が引けず、ローカルプロキシ（127.0.0.1:18080）を HTTPS_PROXY/HTTP_PROXY に指定して初めて token 発行と push が通った。また `mise run verify` は ~/.cache が読み取り専用のため XDG_CACHE_HOME=/tmp/opencode/cache に向けないと mbx/hk が失敗した。`scripts/tests/agent-sandbox.test.sh`（~/.t3/worktrees 配下が読み取り専用・DNS 不通）と `scripts/tests/conductor-watch-ensure.test.sh`（systemd が無い）の 2 件は docsのみの本変更と無関係にこの環境で失敗する。
2. 曖昧だった文: 「負荷試験の対象シナリオ … と合否の目安が文書化されている」（Issue #208 本文・受け入れ条件）— 「目安」に数値を付けるかが不明だったが、未決事項に「目安値の最終値（実装後に実測で決める）」とあるため、仮置きの数値を付けて「仮」明記とし、順序保証・欠落0などの必須条件は数値ではなく性質で書いた。
3. 仮定と解釈: 配置場所は Issue が指定しないため docs/load-testing.md を新設し、README.md のドキュメント一覧と docs/dev-environment.md の nightly 節からリンクした（v0.7 の本文ファイルは verification/verify.py の対応検査対象なので編集しない）。関連情報の「docs/technical/v0.7/body/06-reliability.md」は実在しない（06 は i18n。順序と再接続は 04-data-api-realtime.md）ため、文書内では 04 を参照した。
4. 足りなかった情報・検証手段: block/buzz の Dockerfile.loadgen・perf・prometheus.yml の実体（インストール token の scope が hatai/Pelway 限定で取得できず、Issue 本文の要約のみを根拠にした）。
5. 改善案: `.agents/skills/write-issue/SKILL.md`の「関連情報」に「リポジトリ内のパスは実在を確認する（Issue #208 では 06-reliability.md が実在しなかった）」の一文追加。

使ったスキル: implement-issue
