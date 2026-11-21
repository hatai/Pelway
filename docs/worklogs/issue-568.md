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

## 実装者メモ（round 3・停止）

停止の報告と round 3 のメモは Issue #568 のコメント（「止まりました: 起動に新しい bin（起動口）が要る」）に書いた。push はしていない。

## 実装者メモ（round 4）

対象 head: 7fedcbfb77273d8dab91236f81937056f108b997

1. 詰まった箇所: `.agents/skills/verify-pelway/verify-run` のビルド: 同時に 2 つの run を始めると、一方の `mise exec -- cargo build` が rustc を `/usr/bin` の 1.75.0 に解決してビルドに失敗した（`mise WARN missing: rust@1.99.1` の再同期と重なったときに出る）。`mise current rust` と `rustup which --toolchain` で決めた rustc を `RUSTC` で渡す形にした。
2. 詰まった箇所: 二重起動の確かめ: 道具の版を記録する `mise exec` の呼び出しに約 55 秒かかり、その間に手で起動したインスタンス（25 秒で終わる）が先に終わって、確かめが空振りした。ロックとインスタンスの確認を run の最初に寄せた。
3. 曖昧だった文: 「起動には既存の examples かテストを使い、それを呼ぶ補助スクリプトをスキルのフォルダに置きます。新しい bin が要るとわかったら、そこで止めて報告してください」（開発リードの追加指示）。round 3 で止まった後、開発リードの次の指示は `demo-serve` の手直し（標準入力が閉じたら終わる、など）を求めていた。
4. 仮定と解釈: 3 の指示を、`demo-serve`（example。本番のコードは変えない）を起動口として認めた決定（Issue #568 で挙げた案 A）と読んだ。認めていない読み方（案 B・C）は選ばなかった。Issue に書き残す。
5. 足りなかった情報・検証手段: 生き残りの確認の対象（T3 のプロセス）の名前の決まりが無い。`ps` の comm が `demo-serve`・`pitchfork`・`pelway-agent-wa`・`t3-*` のものに限った。
6. 改善案: `docs/dev-environment.md` の「デモの LLM 接続（#330）」に、`mise exec` の PATH では rustc が `/usr/bin` の版に解決されることがある、を書き足す（`mise WARN missing: rust@1.99.1` の再同期と重なったとき）。

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: 7823fc8587dc740f9aca3c0d3a54bfd27ddcd96b

1. 詰まった箇所: `.agents/skills/verify-pelway/verify-run` の `start_sampler`: 0.1 秒ごとに `ss` を取るサブシェルが、子の標準入力のパイプ（coproc の書き手）を引き継ぐと、run が SIGKILL されたときに子が EOF を受けず残る。サブシェルの最初でその fd を閉じ、親が死んだら止まる形にした。SIGKILL の試しで子が終わることを確かめた。
2. 曖昧だった文: 「段 7 は「bot の投稿が 1 件以上あれば合格」になっていて…下書きの中身で判定する」（バックエンド担当の事前レビュー 5）。中身の判定を「選んだ案A の本文を含む決定後の bot 投稿」とし、さらに保存先から取り直す経路が main に無いので、投稿があっても段 10 は FAIL（未検証）にした。
3. 仮定と解釈: 事前レビュー 4 の「選択 a が保存されたことを HTTP で読み直す」は、確定した決定を 1 件読む GET が無いため、同じ選択の再送（冪等リプレイ）の応答で保存された選択と案A の本文を読み直すと読んだ。
4. 足りなかった情報・検証手段: 確定した決定を読む GET の経路（`pelway-contract` の `DECISIONS_PATH` は未確定の一覧だけ）。
5. 改善案: `.agents/skills/verify-pelway/features/decision.md` の「Gotchas」の「確定した決定を 1 件取る経路は無い」を、経路が足されたら「`GET` で確定した決定を読み直す」に直す。

使ったスキル: implement-issue

## 実装者メモ（round 6）

対象 head: 74c2410aac766f665856681bba70b88c16870dff

1. 詰まった箇所: `.agents/skills/verify-pelway/verify-run` の生き残りの確認: 別の証拠の置き場を渡した同時起動の試しで、1 つめの run が「消えた pid」で FAIL した。毎時 0 分の pitchfork の定期ジョブの短命なプロセスが run の途中で自然に終わったもので、比べる対象を 5 分以上動いている常駐のプロセスに絞った。
2. 曖昧だった文: 「ロックの親が `--evidence-root` で変わります…証拠の保存先とビルドによって変化しないロックの場所・キーを使い」（Codex のレビュー must-fix）。鍵を target の実体ではなく作業ツリーの実体のパスにし、場所を共有 git dir の下に固定した。同じ target を別の作業ツリーが使う場合は、ビルドの前後の「同じ target・同じ実行ファイル」の確認で拒む。
3. 仮定と解釈: Claude のレビュー must-fix（観察が TCP だけ）を、(a) UDP も観察する、で直し、`/proc/<pid>/fd` のソケットを `ss -e` の inode と突き合わせて TCP・UDP と確かめられないソケットを FAIL にした。TCP・UDP 以外の通信路は未検証と書いた。
4. 足りなかった情報・検証手段: TCP・UDP 以外の通信路（unix ソケットなど）の宛先を確かめる手段（strace などは道具の追加と権限が要るので使っていない）。
5. 改善案: `.agents/skills/verify-pelway/SKILL.md` の「5. 後片付け」の生き残りの確認に、T3 の常駐プロセスの comm の一覧（今は `t3-*` だけ）を、T3 の運用文書（`docs/superpowers/specs/agent-team.md`）から参照する形にする。

使ったスキル: implement-issue、review-pr の結果の処理（手順 9）

## 実装者メモ（round 7）

対象 head: f3e26e5aaf67d5fbbb085beac21f556c099eaa75

1. 詰まった箇所: レビュー round 2 の委託: Codex（codex_review）が利用上限で failed。すぐ再委託すると同じ理由で 2 回連続の failed（止まる条件）になるため、上限が解ける時刻の後に 1 回だけ再委託する予約を入れた。
2. 曖昧だった文: 「高リスクで複数担当にしたときは、全員の結果がそろうまで次に進まない」（`.agents/skills/implement-issue/SKILL.md` の手順 8）。Codex が利用上限で結果を出せないあいだに、Claude の must-fix を先に直すかで迷った。直した head を Codex と Claude にそろえて出し直せば両方が同じ head を見るので、先に直した。
3. 仮定と解釈: Claude の round 2 の must-fix（中断した run が PASS になる）を、main が戻ったときだけ完了の印を立て、印が無ければ FAIL にする、で直した。SIGTERM で中断した run で FAIL になることを確かめた。
4. 足りなかった情報・検証手段: Codex の利用上限が解ける時刻（失敗の文言の「2:00 PM」だけが手がかり）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「委託の子が `failed` を報告したら」に、「提供元の利用上限による failed は、上限が解ける時刻の後に再委託し、連続の数に入れない」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 8・停止）

停止の報告と round 8 のメモは Issue #568 のコメント（「止まりました: 3 回目のレビュー結果に must-fix が残った」）に書いた。

## 実装者メモ（round 9）

対象 head: 943032b2772f775bfa3f173dfbd6d3bafa2450c9

1. 詰まった箇所: 再開の前: PR の head が手元の 0ce296da から 8647307d（hatai による main のマージ）に進んでいた。fast-forward で合わせ、取り込まれた implement-issue の変更（`agent:working` と取得コメントによる占有）に従って Issue を取った。`mise run verify` は main の取り込みでテストが増え、590 秒の時間切れで一度止まったため、時間切れを延ばして回し直した（322 秒で通った）。
2. 曖昧だった文: 「残っている must-fix 3 件はこの PR の中で直してください（ラウンド上限は撤廃済み・なくなるまで修正と再レビューを継続）」（指揮役経由の濱田さんの許可）。Issue に hatai 名義のコメントとしては見当たらず、指揮役のスレッドからの伝達として受けた。
3. 仮定と解釈: Codex の must-fix 2（置き場の下のシンボリックリンク）を、run のディレクトリを毎回新しく 700 で作り、親とそのディレクトリの実体が予定のパスと一致しなければ止まる、と読んだ。置き場の下のすべての既存ディレクトリの権限を直す読み方は選ばなかった（他の run の証拠を書き換えることになるため）。
4. 足りなかった情報・検証手段: `ss` の取得を故意に失敗させる手段（実機では起こせず、`observation_errors` のテストと印の行の扱いで確かめた）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「main を取り込んだ後の `mise run verify` は 5 分を超えることがあるので、コマンドの時間切れを 10 分より長くとる」を足す。

使ったスキル: implement-issue
