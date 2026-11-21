# Issue #568 の作業ログ

## 実装者メモ（round 1）

対象 head: a60ee682449e91160c8f4f8cc3cb7be06b00c182

1. 詰まった箇所: 手順 4（起動の値を確かめる段）: main には起動して動かし続けられるサーバー・クライアントのバイナリが無かった（`main.rs` は dev-llm-eval と agent-watch だけ、`demo-log-emit` は要求を流して自分で終わる）。公開の `serve_with_store_and_runner` と台本モードだけを使う起動口 `crates/pelway-http/examples/demo-serve.rs` を足し、本番相当の環境に無い準備として PR に挙げた。
2. 詰まった箇所: 証拠の置き場: Issue は「保存先（リポジトリ外、run ごと・sha 名つき）」、implement-issue は「worktree の外に書き込まない」。`~/.local/state` への試し書きは auto mode に拒否された。既定は `${XDG_STATE_HOME:-$HOME/.local/state}/pelway-verify` のまま、`--evidence-root` で受け、自分の run はセッションの scratchpad（リポジトリ外・書き込み可）に置いた。
3. 曖昧だった文: 「値はすべてこのリポジトリで実際に確かめたものにする。main でまだ動かない部分は「失敗（理由）」と書き、確かめ方を弱めない」（開発リードの指示）。起動口が無いことを「失敗」と書くか、起動口を足すかで迷い、完了条件の「代わりの環境で確かめた場合は…準備手順を列挙する」を根拠に後者を選んだ。メール下書きとクライアント画面は足さずに「失敗（理由）」とした。
4. 仮定と解釈: Issue に「受け入れ条件」「採用する方式」の見出しは無く、「完了条件」と「SKILL.md に入れる章」を受け入れ条件、pstack `create-verification-skill` の構成を採用する方式として読んだ（gh search による方式比較はしていない）。
5. 足りなかった情報・検証手段: 外に出ていないことの観察は `ss` のスナップショット（要求ごと）と `/proc/<pid>/fd` で、接続の全記録（strace・tcpdump）は権限とグローバルな道具の追加を避けて使っていない。
6. 改善案: `docs/dev-environment.md` の「デモの LLM 接続（#330）」の「`mise exec -- fnox exec -- <デモ起動コマンド>`」に、`cargo build --offline --locked -p pelway-http --example demo-serve`（台本モード固定）を書き足す。ただし demo-serve は環境変数を読まないため、実キーでの起動口は別に要る。

使ったスキル: implement-issue、write-pull-request、writing-for-agents（`.agents/skills/writing-for-agents/SKILL.md`）

## 実装者メモ（round 2）

対象 head: c09fa0ae96f814256a8a1ce513ed34ac38f74b93

1. 詰まった箇所: `.agents/skills/verify-pelway/verify-run` の `env_check`: 証拠に変数名 `PELWAY_LLM_*` を書いたら、run のログへ流した demo-check の leak-phrases が `[ident]` で検出した。証拠には変数名を書かず、`env_llm_key=absent` のようなラベルだけを書く形にした。
2. 詰まった箇所: 同ファイルの `http`: 要求ヘッダーをコマンド置換で組むと末尾の改行が落ち、要求が壊れて 408 になった。ヘッダーの終わりを送るときに改行を足す形にした。`ss -tnp` を PID だけで絞ると接続が閉じた後に 0 行になるため、待ち受けのアドレスを含む行も取る（`ss -tanp`）形にした。
3. 曖昧だった文: 「台本モードで動いたことは、合成応答が出たことと、接続先を知らせるログ行で確かめる」（開発リードの追加指示 1）。接続先の行は台本モードでも Z.ai の URL を書くので、接続していないことは `network.txt` の集計（loopback 以外 0 件）で別に確かめると読んだ。
4. 仮定と解釈: 「ログと証拠に、投稿の本文や秘密を出さない」を、証拠の `body`・`question` を長さだけ残して伏せることと読んだ。依頼の本文に demo-check の canary を埋め、伏せ漏れを canary 段で検出できるようにした。案A・案B の一致は伏せる前のメモリ上で確かめ、PASS 行だけを残す。
5. 足りなかった情報・検証手段: demo-check の各段は走査範囲が固定（`target/`・`logs/` など）で、任意のログを渡す引数が無い。run の証拠の写しを `demo-check-input/target/demo/` に置き、`DEMO_CHECK_ROOT` で流した。
6. 改善案: `scripts/demo-check/lib.sh` の冒頭の「走査範囲」に、`DEMO_CHECK_ROOT` で任意のディレクトリの `target/demo/` を走査させる使い方（verify-pelway の run のログ）を書き足す。

使ったスキル: implement-issue、write-pull-request
