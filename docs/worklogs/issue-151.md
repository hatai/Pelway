# Issue #151 実装メモ: 実装スレッドを sandbox で動かす

## 要求の整理

Issue 本文の受け入れ条件（案）は 4 項目。hatai のコメントで 1 項目目の
ネットワーク許可リストは「厳しくしすぎると調査が難しくなるので現時点
では許可リストにしない」に修正された。残りは:

- worktree と build cache 以外への書き込みを OS レベルで拒否し、sudo を
  使えないこと
- T3 の権限・OS sandbox・コンテナのどれで実現するかを opencode と Devin
  それぞれ確かめ、設定と確認方法を `docs/dev-environment.md` に書く
- worktree 外への書き込みが拒否されることを確かめる手順を用意する

## 採用した方式

両エージェントとも、リポジトリ同梱の `scripts/agent-sandbox/sandbox-shell`
（bubblewrap）による mount 名前空間で遮断する。T3 の権限・各 CLI の
permissions は補助層で、境界は mount 側が担う。

- **opencode**: グローバルプラグイン `pelway-sandbox`（V2 API）。shell フック
  `create.before` で全コマンドを `sandbox-shell` 経由にし、ファイルツール
  `execute.before` で worktree 外の書き込みと秘匿パスの読み取りを拒否。
  適用は `~/.t3/worktrees/` 配下のセッションのみ
- **Devin CLI**: `acpRegistry_devin` の `commandPath` に `devin-impl`
  ラッパーを指定し、`devin acp` プロセス自体を `sandbox-shell --wrap` で
  包む。Devin 内蔵の `--sandbox` は不採用（下記の検証結果）

## 検討と却下した選択肢

- **Devin 内蔵 sandbox（`DEVIN_SANDBOX=true`）**: `allowed_domains` を
  空にしても exec の通信が組み込みプロキシ経由で github.com 系のみに
  なり（`*`・`**`・`*.**` は全拒否）、crates.io・npm・rustup・一般
  サイトに届かない。ネットワーク開放の要請と両立しないため不採用。
  プロジェクト側 `.devin/config.json` の `Write(/**)` が書き込み範囲を
  広げないことも実機で確認した
- **opencode 公式の機構**: opencode に OS sandbox はなく、シェルは
  ホスト権限で動く。プラグインフックでコマンド差し替え・ツール遮断が
  できることを実機で確認し採用
- **コンテナ**: T3・fnox・agent-gh との統合が大掛かりになるため、
  devin_review で実績のある bwrap を採用

## 主な設計判断

- `/usr`・`/etc`・`/home` 等の top-level のみを read-only で明示列挙。
  `--ro-bind /` は `/mnt/c` 等の submount まで写すため使わない（Issue
  #98 の再現防止）
- 共有 git dir はコミットのため書き込み可、`config`・`hooks/` は ro-bind
- `commit.gpgsign=false` を env 注入（`~/.ssh` を隠すため SSH 署名は
  必ず失敗する。push 後に agent-gh-resign が署名を作り直す前提）
- `MISE_RUST_MR_BOXINGTON=0` で mbx を無効化し `target` symlink を実
  ディレクトリに戻す（Issue #97 の再現防止）
- sandbox 無効化経路を塞ぐため `opencode.json[c]`・`.opencode/`・
  `.devin/` を ro-bind。bwrap は存在しないマウントポイントをホストに
  作るため、対象ファイルをリポジトリに同梱した
- bwrap は広い bind を後から重ねると子の ro-bind が実質無効化される
  （実機確認）ため「広い→狭い」の順でマウントする

## 変更ファイル

- `scripts/agent-sandbox/sandbox-shell` — 共通 bwrap ラッパー
- `scripts/agent-sandbox/opencode-plugin/index.js` — opencode プラグイン
- `scripts/agent-sandbox/devin-impl` — `acpRegistry_devin` 用ラッパー
- `scripts/agent-sandbox/devin-config.json` — devin-impl の permissions
- `scripts/setup/agent-sandbox.sh` — ホストへの導入スクリプト
- `scripts/tests/agent-sandbox.test.sh` — script-test（mise.toml に登録）
- `opencode.json`・`.opencode/.gitkeep`・`.devin/config.json` — マスク用同梱
- `docs/dev-environment.md`・`docs/superpowers/specs/agent-team.md` — 仕様・手順

## 検証結果

- `bash scripts/tests/agent-sandbox.test.sh`: 全項目合格（worktree 書込・
  コミット可、ホーム・/etc・git config/hooks・無効化パス拒否、秘匿パス
  遮蔽、sudo 失敗、/mnt 不在、env 除去、wrap モード）
- プラグインの node 直接検証: worktree セッションでのみ発火、write 系の
  外部・保護パス拒否、read 系の秘匿パス拒否、非 worktree では非活性
- `opencode run` の実セッション（worktree）: シェル経由の `$HOME` 書込が
  `Read-only file system`、`sudo -n true` 失敗、worktree 書込成功、
  write ツールの外部書込が `pelway-sandbox: write: worktree 外への
  書き込みは禁止` で拒否
- `devin-impl`（wrap 方式）の実セッション: `curl https://example.com` が
  200（ネットワーク開放）、`agent-gh dev api /rate_limit` 成功、
  `$HOME` 書込 `Read-only file system`、worktree 書込成功
- 実機で得た修正: devin は `--config` のファイル自体を atomic write で
  更新するため `~/.config/devin-impl` を丸ごと書き込み可にした。
  credentials.toml も同様に `~/.local/share/devin` ごと必要だった
- 比較検証: Devin 内蔵 sandbox では `allowed_domains` 空でも exec が
  github.com 系のみ（example.com/crates.io/npm タイムアウト）

## 残件・運用時の確認

- ホスト側の反映（`~/.t3/userdata/settings.json` の
  `acpRegistry_devin.config.commandPath` 追加 + T3 再起動）は人間の
  手順。`scripts/setup/agent-sandbox.sh` が案内を出す
- T3 の実スレッドでの確認（opencode と `acpRegistry_devin`）は settings
  反映後の初回スレッドで実施し、`docs/dev-environment.md` の記録欄へ
  結果を書き込む
- 他リポジトリの worktree では初回実行で空の `opencode.json`・
  `.opencode/`・`.devin/` が作られる（bwrap がマウントポイントをホストに
  作成するため。Pelway では同梱済みのため発生しない）
- credentials.toml と fnox の age 鍵は sandbox 内から読める（agent-gh・
  devin 自身が必要とするため）。ネットワーク開放の方針上、理論的な
  外部送信経路は残り、第一目的はホストへの書き込み・昇格の防止とする
