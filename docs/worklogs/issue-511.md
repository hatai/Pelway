# Issue #511 の作業ログ

## 実装者メモ（round 1）

対象 head: bd689af95e3cd9ec707c4b310d3bab680f81c8d4

1. 詰まった箇所: `pitchfork.toml` の `boot_start`: 上流 v2.33.1 の `start_boot_daemons`（`src/supervisor/autostop.rs`）は `all_merged_all_namespaces` で設定を集めるが、`~/.config/systemd/user/pitchfork.service` に作業ディレクトリが無く、`~/.config/pitchfork/config.toml` も無い。そのため、リポジトリ側の設定だけでは起動時に読まれない。本体 checkout を `[namespaces]` に登録する手作業が要ると分かり、hatai に確かめて（会話で「推奨で進める」）手順に足した。
2. 曖昧だった文: 「`pitchfork.toml` の agent-watch に、supervisor が上がったら自動で起動する設定を足す。」（Issue #511「リポジトリ側でやること」）— 設定を足せば自動起動するという前提で書かれていたが、namespace の登録が別に要るため、文書に PC 側の手順として書いた。
3. 仮定と解釈: 起動時の supervisor が各 worktree の定義からも起動を試みる点は、#510 の拒否（終了コード 0）で足りると解釈し、`general.worktree` は変えなかった（hatai の判断）。namespace を登録したときの daemon の修飾名は確かめきれないため、文書のコマンドは「本体 checkout の直下で実行する」形にした。
4. 足りなかった情報・検証手段: WSL の再起動のあとに実際に起動するかは、この環境では確かめられない（未検証。hatai の確認待ち）。本体 checkout は今 `agent/510-agent-watch-start-guard` にあり、マージ後に main へ切り替える必要がある。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「リポジトリ外の設定」の項に「外部ツールの設定の効き方（どの設定ファイルが、どの起動の仕方で読まれるか）は、起票の前に上流の資料かソースで確かめる」を足す。

使ったスキル: implement-issue、write-pull-request
