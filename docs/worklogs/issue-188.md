# Issue #188 の作業ログ

## 実装者メモ（round 1）

対象 head: f7f731b（署名付け替え後。作業ツリー上のコミット ed0d3fe と同一内容）

1. 詰まった箇所: `scripts/agent-gh`（手順 1）: `gh token generate` が DNS 解決失敗で exit 3 を返した。環境の `/etc/resolv.conf` が `/mnt/wsl/resolv.conf` への dangling symlink になっており、ローカルプロキシ（127.0.0.1:18080）を `HTTPS_PROXY`/`HTTP_PROXY` に設定すると到達した。以降の agent-gh・mise 呼び出しにも同じ環境変数と `XDG_CACHE_HOME=/tmp/opencode/cache`（読み取り専用の `~/.cache` 回避）を付けた
2. 曖昧だった文: 「ネットワークは `scripts/agent-gh dev` 経由の GitHub だけ使う。」（`.agents/skills/implement-issue/SKILL.md` の決まり）— Issue #188 の「採用する方式」が「公式資料ベースで比較し」と、composio.dev・pipedream.com など GitHub 外の公式資料での比較を要求する
3. 仮定と解釈: 上の決まりは GitHub 操作の名義（gh/git は agent-gh を通す）を定めるものと解釈し、公式資料の取得はハーネスの webfetch、IP 割り当ての確認だけ Amazon 公式 `ip-ranges.json` をローカルプロキシ経由の curl で行った。選ばなかった解釈: 文字どおり GitHub 限定と読み「リポジトリ外の成果物に到達できない」の止まる条件で止まる。もう1つ: 受け入れ条件の確かめ方が3条すべて「調査文書を目視で確認」なので、自動テストは書かず write-issue skill の「確認の手順」形式（前例 #200 と同じ docs-only PR）に従った
4. 足りなかった情報・検証手段: `mise run verify` のうち `scripts/tests/agent-sandbox.test.sh`（DNS・読み取り専用ホーム依存の4 FAIL）と `scripts/tests/conductor-watch-ensure.test.sh`（behavior-fallback の1 FAIL）が本環境で失敗する。変更を `git stash -u` した pre-change の tree で同一失敗（両方 rc=1）を確認し、docs 2ファイルの本変更不起因を確かめた。`verify-lint`・`verify-static`・`mise run test` は rc=0
5. 改善案: `docs/dev-environment.md` に「HTTP プロキシ経由でしか外へ出られない環境での agent-gh・mise の実行方法（HTTPS_PROXY と XDG_CACHE_HOME の設定）」を追記する

使ったスキル: `implement-issue`、`write-pull-request`

## 実装者メモ（round 2）

対象 head: なし（round 2 の実体はマージコミットのみ。メモをマージコミットに同梱したため、メモを含むコミットの親に当たる実装コミットは存在しない）

1. 詰まった箇所: `docs/research/README.md`: main 側（#368 の `revocation-generation-model.md`）と本 PR の索引行が同じ位置に追加されて競合。両方の行を残し、main 側の行を先に置いて解消した
2. 曖昧だった文: 「Approve の後は PR に push しない。実装者メモのみのコミットでも head が動き、command-team のマージ規則によりレビューのやり直しになる。」（`.agents/skills/implement-issue/SKILL.md` 手順 9）— 一方で指揮役の修正依頼（[PR #388 コメント](https://github.com/hatai/Pelway/pull/388#issuecomment-6923152042)）は main の取り込みと push を明示的に求め、「レビュー有効性は指揮役が head で確認します」と判断を指揮役側に予約している
3. 仮定と解釈: 実装者メモ（round 2）をマージコミットそのものに同梱して push した。これによりレビュー済み SHA 4717dcc 以降の non-merge コミットは 0 件のまま（command-team のマージ規則 `git rev-list 4717dcc..<head> --no-merges ^origin/main` が 0 件の条件を保持）になり、有効な Approve 2 件が失効しない。メモだけの独立コミットは選ばなかった（head が動きレビューのやり直しを強制するため）。round 2 に実装の変更は無く、main の取り込みと索引の競合解消、本メモのみ
4. 足りなかった情報・検証手段: `mise run verify` は round 1 と同じ環境変数（HTTPS_PROXY・XDG_CACHE_HOME）で実行。script-test の2失敗（`agent-sandbox.test.sh`・`conductor-watch-ensure.test.sh`）は引き続き本環境の読み取り専用 `/home`・DNS 不在に起因し、変更不相関
5. 改善案: round 1 と同じ（`docs/dev-environment.md` へのプロキシ環境の実行方法の追記）

使ったスキル: `implement-issue`
