# Issue #151 実装メモ: 実装スレッドを sandbox で動かす

## 要求の整理

Issue 本文の受け入れ条件（案）は 4 項目。hatai のコメントでネットワーク
許可リストは「現時点では許可リストにしない」に修正された。残りは
worktree と build cache 以外への書き込み拒否・sudo 不可、opencode と
Devin での実現方式の確認と `docs/dev-environment.md` への記録、
worktree 外書込拒否の確認手順。

## 採用した方式

両エージェントとも、リポジトリ同梱の `scripts/agent-sandbox/sandbox-shell`
（bubblewrap）による mount 名前空間で遮断する。T3 の権限・各 CLI の
permissions は補助層で、境界は mount 側が担う。

- **opencode**: グローバルプラグイン `pelway-sandbox`（V2 API）が shell
  フックで全コマンドを `sandbox-shell` 経由にし、ファイルツールの
  worktree 外書込と秘匿パス読取を拒否（`~/.t3/worktrees/` 配下のみ）
- **Devin CLI**: `acpRegistry_devin` の `commandPath` に `devin-impl`
  を指定し、`devin acp` プロセス自体を `sandbox-shell --wrap` で包む

## 検討と却下した選択肢

- **Devin 内蔵 sandbox**: `allowed_domains` 空でも exec の通信が組み込み
  プロキシ経由で github.com 系のみ（ワイルドカードは全拒否）になり、
  ネットワーク開放の要請と両立しない。`.devin/config.json` の
  `Write(/**)` が書き込み範囲を広げないことは実機確認した
- **opencode 公式の機構**: OS sandbox はなくシェルはホスト権限。
  プラグインフックで差し替え・遮断できることを実機確認し採用
- **コンテナ**: T3・fnox・agent-gh との統合が大掛かりなため不採用

## 主な設計判断

- top-level のみを ro-bind で明示列挙。`--ro-bind /` は `/mnt/c` の
  submount を写すため使わない（Issue #98 の再現防止）
- 共有 git dir は書き込み可、`config`・`hooks/` は ro-bind。
  `commit.gpgsign=false` を env 注入（`~/.ssh` は隠すため署名は必ず
  失敗し、push 後に agent-gh-resign が作り直す前提）
- `MISE_RUST_MR_BOXINGTON=0` で mbx を無効化し `target` symlink を実
  ディレクトリに戻す（Issue #97 の再現防止）
- `opencode.json[c]`・`.opencode/`・`.devin/` を ro-bind で無効化経路を
  塞ぐ。bwrap は存在しないマウントポイントをホストに作るため同梱した
- bwrap は広い bind を後から重ねると子の ro-bind が実質無効化される
  （実機確認）ため「広い→狭い」の順でマウントする

## 変更ファイル

`scripts/agent-sandbox/`（wrapper・プラグイン・devin-impl・devin 設定）、
`scripts/setup/agent-sandbox.sh`、`scripts/tests/agent-sandbox.test.sh`、
マスク用同梱の `opencode.json[c]`・`.opencode/`・`.devin/config.json`、
`docs/dev-environment.md`・`docs/superpowers/specs/agent-team.md`。

## 検証結果

- `bash scripts/tests/agent-sandbox.test.sh`: 全項目合格（書込・コミット
  可、ホーム・/etc・git config/hooks・無効化パス拒否、秘匿パス遮蔽、
  sudo 失敗、/mnt 不在、env 除去、wrap、プラグイン実体パス検査）
- `opencode run` 実セッション: `$HOME` 書込が `Read-only file system`、
  sudo 失敗、worktree 書込成功、write ツールの外部書込が
  `pelway-sandbox:` のエラーで拒否
- `devin-impl` 実セッション: `curl https://example.com` が 200（開放）、
  `agent-gh dev api /rate_limit` 成功、`$HOME` 書込拒否、worktree 書込可
- devin は `--config` のファイル自体を atomic write で更新するため
  `~/.config/devin-impl` と `~/.local/share/devin`（credentials.toml）を
  書き込み可にした

## 残件・運用時の確認

- ホスト側の反映（`~/.t3/userdata/settings.json` に `acpRegistry_devin.
  config.commandPath` 追加 + T3 再起動）は人間の手順
- T3 の実スレッドでの確認は settings 反映後の初回スレッドで実施し、
  `docs/dev-environment.md` の記録欄へ書き込む
- 他リポジトリでは初回実行で空の `opencode.json`・`.opencode/`・
  `.devin/` が作られる（bwrap がマウントポイントをホストに作るため）
- credentials.toml と fnox の age 鍵は sandbox 内から読める（agent-gh・
  devin が必要とするため）。ネットワーク開放の方針上、理論的な外部送信
  経路は残り、第一目的はホストへの書き込み・昇格の防止とする

## 実装者メモ（round 1）

対象 head: a9f58d455a3dfc1a6211090c1b84c31d5d3acaa0（このメモのコミットの親。レビュー指摘の修正を含む実装コミット）

1. 詰まった箇所: scripts/agent-sandbox/sandbox-shell: bwrap の「広い bind を後にすると配下の ro-bind が無効化される」順序依存と、`--ro-bind /` が WSL の /mnt/c submount を写す問題で、明示マウントと広→狭の順に落ち着くまで実機検証が要った
2. 曖昧だった文: 「厳しくしすぎると調査などが難しくなるので、現時点では許可リストにはしない」（Issue #151 の hatai コメント）
3. 仮定と解釈: 許可リスト非採用を「Devin 内蔵 sandbox（GitHub-only egress）を使わない」と解釈し、devin プロセスを bwrap で包む方式を選んだ。選ばなかった解釈: 内蔵 sandbox + 広い許可リスト（実機で空指定が GitHub-only になると確認したため）
4. 足りなかった情報・検証手段: opencode プラグイン V2 API（`ctx.shell.hook`/`ctx.tool.hook`）の形状はドキュメント未整備で、opencode 本体のソースと実機フック実験で確かめた
5. 改善案: `docs/dev-environment.md` の「実装担当の sandbox」に「ファイルツールの検査は実体パスで行う（symlink 越しの脱走対策）」を足した（round 1 のレビュー指摘対応として適用済み）

使ったスキル: implement-issue

## 実装者メモ（round 2）

1. 詰まった箇所: hooks.v1.json の `command` に引数付きパスを書くと shell 解釈の問題で起動に失敗したため、引数なしの `devin-hook` エントリポイントを用意。また初版の `python3 -` は stdin をプログラムに使うためイベント JSON が読めず `python3 -c` に直した
2. 曖昧だった文: 「`updatedInput` は `PreToolUse` `hookSpecificOutput` 内に返す」（devin CLI 公式ドキュメント hooks/overview）
3. 仮定と解釈: devin プロセスは credentials.toml を読む必要があるため outer namespace では `~/.local/share/devin` を見せたままにし、exec コマンドだけをネストした sandbox（同パスは tmpfs マスク）に入れる。選ばなかった解釈: credentials.toml の ro-bind（atomic write で起動に失敗）
4. 足りなかった情報・検証手段: hook の command の起動形式と `$HOME` 展開の有無はドキュメント未記載で、debug hook（stdin/stdout を tee 記録）で実機確認した
5. 改善案: `docs/dev-environment.md` の devin-impl の行にフックによる資格情報分離を追記した

使ったスキル: implement-issue
