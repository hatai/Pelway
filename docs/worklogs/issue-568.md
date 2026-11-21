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

## 実装者メモ（round 10）

対象 head: 6bfddcce51c4b8ee391dff78f68a9c2a2ed0a8fa

1. 詰まった箇所: レビュー round 4: Claude（claude_review）が利用上限で一度 failed し、上限が解けた後に自動で再開して approve を出した。Codex は request-changes（must-fix 5）で判定が割れたため、request-changes 側を優先して直した（`.agents/skills/implement-issue/SKILL.md` の手順 9）。
2. 曖昧だった文: 「nit は、`pelway-review` のレビューの nit のうち行頭に `[文書のずれ]` の印があるものだけを一緒に直し」（`.agents/skills/implement-issue/SKILL.md` の手順 9）。approve 側（Claude）のレビューの印の nit も、request-changes の修正 round で一緒に直してよいかで迷った。どちらも `pelway-review` のレビューで、Approve のときも印の nit は `pr ready` の前に直すと決まっているので、同じ round で直した。
3. 仮定と解釈: Codex の must-fix 5（最後の PASS をハッシュの照合の前に出す）を、verdict を summary に書いてからハッシュを作って照合し、照合できたときだけ標準出力に出す、照合できなければ summary の verdict を FAIL に書き換えて SHA256SUMS を消す、と読んだ。verdict を別のファイルにしてハッシュの外に置く読み方は選ばなかった（verdict がハッシュで守られなくなるため）。
4. 足りなかった情報・検証手段: ハッシュの作成の失敗を実機で起こす手段。`SHA256SUM` で道具を差し替えるテスト（`false`）で確かめた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「複数のレビュー担当の判定が割れたら」に、「approve 側のレビューの `[文書のずれ]` の nit も、その修正 round で一緒に直す」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 11）

対象 head: dd8360c0e2cbfaf26c0d89993925156d804fe7de

1. 詰まった箇所: `.agents/skills/verify-pelway/verify-run.test.sh` の引数の検査: verify-run は引数より先に道具の有無と自分の環境の禁止の変数を確かめるため、`PELWAY_DATABASE_URL` を設定したまま `mise run verify-test` を回す nightly では exit 1 になり、テストが落ちていた。直す前のテストを `PELWAY_DATABASE_URL=x` で回して 2 件落ちることを確かめてから直した。
2. 曖昧だった文: 「PR 本文の「レビューと QA」には round の履歴を書かず、実装者メモ（`docs/worklogs/issue-<N>.md`）を指す 1 行と、最後に検証した sha だけを書く」（Claude の round 5 の nit が引いた implement-issue の手順 7）。このブランチの `.agents/skills/implement-issue/SKILL.md` に見当たらず、印の無い nit なので直さなかった。
3. 仮定と解釈: Codex の round 5 は利用上限で failed したが、前回と同じく上限が解けた後に自動で再開する見込みとして、再委託しなかった。round 6 の委託で Codex を出し直す。
4. 足りなかった情報・検証手段: nightly の環境を手元で再現する手段（`PELWAY_DATABASE_URL` を置いて script-test を回す形で代えた）。
5. 改善案: `.github/workflows/nightly.yml` の冒頭のコメント「script-test を走らせない」を、`verify-test` が script-test を含む事実に合わせる（workflow の変更は人間が行う）。

使ったスキル: implement-issue

## 実装者メモ（round 12）

対象 head: 6191167e3bcad2d2db4ed3c49dcd2ea3afaf4c33

1. 詰まった箇所: 手順 6: `mise run verify` の終了コードを確かめる前に、コミットと push を同じコマンドで続けてしまい、exit 101 の verify のまま 6191167e を push した。落ちたのは `crates/pelway-llm/tests/gateway.rs` の `timeout_after_send_is_outcome_unknown_without_resend`（単独で 3 回回すと 3 回とも通る）。2 回目の verify は cargo が rustc を `/usr/bin` の 1.75.0 に取り違えて失敗（mise の rust の再同期と重なる環境の問題）。3 回目は通った（`script-test: 37 tests ok`）。この round の差分はスキルの 4 ファイルだけで、Rust のコードには触れていない。CI の必須チェックは 6191167e で 2 つとも通った。
2. 曖昧だった文: 「PR 本文の「レビューと QA」には round の履歴を書かず、実装者メモ（`docs/worklogs/issue-<N>.md`）を指す 1 行と、最後に検証した sha だけを書く」（main の `.agents/skills/implement-issue/SKILL.md`）。このブランチの版には無く、round 11 では見落とした。main の版に従い、PR 本文から round の履歴と前の round の結果を外した（履歴はこのメモが正本）。
3. 仮定と解釈: 生き残りの確認の T3 本体を、プロセス名ではなく実行ファイル名 `t3`（`scripts/agent-worktree-prune` が T3 サーバーとして探す名前）で拾うと読んだ。プロセス名は `node-MainThread` で、名前の一覧に足す読み方は選ばなかった（ほかの node のプロセスも拾うため）。
4. 足りなかった情報・検証手段: `pelway-llm` の時間に左右されるテストが、重い並列の verify の中で落ちる頻度。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 のコード例を、`mise run verify` が失敗したらそこで止まる形（`mise run verify && scripts/agent-gh dev git add ...`）にする。

使ったスキル: implement-issue

## 実装者メモ（round 13）

対象 head: 9aef851ba27dc1f05037f95df3990c5f70993b2b

1. 詰まった箇所: この実行環境（Devin の sandbox）のマウント名前空間: `/tmp` と `/home` が nobody 所有のため、`dir_chain_safe` の「置き場より上は自分か root」にどの一時領域も合わず、`make_run_dir` の試験が落ちる（HEAD 版でも再現＝環境起因。`TMPDIR=/dev/shm/<dir>` では全通過）。実 run は `--evidence-root` を `/dev/shm` に向け、run ディレクトリを標準の置き場に写して `sha256sum -c` で照合した。前任の環境では合っていた。あわせて自分の run で 2 件見つけて直した: ac0621f6 の mail-draft run が「消えた pid 544960」（T3 本体の子 `t3 acp-mcp-bridge`。親が t3 で run の途中に自然終了）で FAIL→ T3 本体を「実行ファイル名が t3 で親が t3 ではないもの」に絞る（59791cb7）。私の run で demo-serve に端末の `/dev/ptmx` が継承され「開いているファイル」FAIL → `env -i` は fd を清めないので、起動の直前に標準入出力以外の fd を閉じる（9aef851b）。
2. 曖昧だった文: 「生き残りの確認: …と T3 本体（実行ファイル名が `t3`。プロセス名は `node-MainThread` などになる）の PID を取り」（`.agents/skills/verify-pelway/SKILL.md` の「5. 後片付け」）。実行ファイル名だけでは T3 の子も拾う。「親が t3 ではないもの」と絞った。
3. 仮定と解釈: T3 の子は 5 分以上動いていても比べないと読んだ（短命の常駐に近い子を網羅する comm の一覧は作れず、親が t3 かどうかで機械に分けられる）。comm の一覧に `t3-acp-mcp-bridge` を足す読み方は選ばなかった。
4. 足りなかった情報・検証手段: この sandbox のマウント構成の仕様（/dev/sdd の分割マウントと exec ごとの /dev/shm）。
5. 改善案: `.agents/skills/verify-pelway/SKILL.md` の前提に「証拠置き場の祖先が自分か root の持ち物でない環境（一部の sandbox）では run と試験は止まる」を足す。

使ったスキル: implement-issue、verify-pelway

## 実装者メモ（round 14）

対象 head: 5309e9580c8fdda6b5204fad5ef963a0dea5da58

1. 詰まった箇所: round 8 の委託が両担当ともプロバイダの使用量制限で起動に失敗した（Claude は解除後に自動再開して approve、Codex も自動再開で request-changes）。Codex の must-fix 2 件: 同じビルド先を共有する別の作業ツリーの run 同士では、ビルドの後に取っていた 2 つめのロックでは、先の run がハッシュを取る前に後の run が実行ファイルを置き換えうる → ビルド先の実体のパスを鍵にしたロックをビルドの前から run の終わりまで持つ（`try_lock`、ロックは cleanup で外す。鍵は `target_real`）。定期観察（サンプラ）が停止の要求より前にほかのシグナルで消えても `wait` の 137 を失敗にしていなかった → 意図した停止（143）以外の終了は全部 OBS_FAILED。あわせて試験を足した: 停止の要求より前にシグナルで終わったサンプラ・自分で抜けたサンプラが失敗になること、`try_lock` の排他と再取得、bin ロックが `cargo build` より前に取られること（ソースの並びの機械検査）。
2. 曖昧だった文: 「二重起動の確認（ビルドの前、run の最初）」（`.agents/skills/verify-pelway/SKILL.md` の 1 章）──「ビルドの前」が作業ツリーのロックだけを指すのか、ビルド先のロックも含むのか曖昧で、ビルド先のロックをビルドの後に取る実装を生んだ（同じビルド先の実行ファイルが、ハッシュを取る前に置き換えられても気づけない）。Codex の指摘文は行番号が差し替え後のものだったが、ソースと照合して「ビルド後の bin ロック取得」「stop_sampler が 3 だけを失敗にする」を指すと読んだ。
3. 仮定と解釈: ロックの鍵は「実行ファイルの置き場」ではなく「ビルド先の実体（`realpath -m`）」に広げた（ビルドの前には実行ファイルの置き場がまだ分からず、target がリンクになる前でも同じ鍵を引ける）。印の無い nit 2 件（`compgen -G` の glob、偽 hasher の `-c` 探し）はこの round では直さなかった。
4. 足りなかった情報・検証手段: 本当に別のコミットから同じビルド先で並行ビルドする結合試験（ロックの試験と並びの検査で代えた）。
5. 改善案: `.agents/skills/verify-pelway/verify-run.test.sh` に、偽物のサンプラではなく実物の `start_sampler` と `stop_sampler` の組で「停止の要求に終了コード 4 で応える」ことを確かめる試験を足す（trap の番号と判定がずれても気づけない構造だった）。

使ったスキル: implement-issue、verify-pelway

## 実装者メモ（round 15）

対象 head: bdc760c2c028c44d722e176c28fd454e5023c226

1. 詰まった箇所: Codex の round 9（must-fix 3・印の nit 1）の直し: (a) 停止の要求より前にほかの SIGTERM で終わったサンプラを、終了コード 143 だけではこの run が止めたものと区別できない → サンプラは TERM を trap して終了コード 4 で応答し、stop_sampler は「要求の前に消えている（ゾンビ含む）」と「応答が 4 ではない」の両方を OBS_FAILED にする。(b) `--evidence-root` の有無によらず `<git-common-dir>/pelway/locks` は共有の置き場なのに、置き場の鎖の検査が証拠側だけだった → `locks_dir_ready` で `pelway` と `pelway/locks` の鎖を `dir_chain_safe` で検査し、pid の表示は `lock_owner` で「通常ファイル・リンクではない・数字だけ」のものに限る。(c) 相対の `CARGO_TARGET_DIR` を呼び出し側の cwd から解決していたため、作業ツリーのサブディレクトリからの run と絶対パスの run が同じビルド先で別のロックを引きうる → `target_dir_of` で `TOP` から解決する（`cd "${TOP}"` してから Cargo を呼ぶのと同じ基準）。あわせて、この sandbox では `/home` が nobody 所有に見えるため (b) の検査を加えた版では実 run が `locks_dir_ready` で止まる → 検査を緩めず、bwrap で `/home` 以下の必要な範囲だけを自分所有に見せた名前空間で run した。bwrap 内は DNS が切れる（`/etc/resolv.conf` が `/mnt/wsl/resolv.conf` へのリンクで、リンク先を bind していなかった）と `~/.rustup` の bind だけでは mise の shim が channel の同期に落ちるため、`/mnt/wsl` を bind し、PATH の先に `~/.cargo/bin`（本物の rustup、`which` はローカル解決）を置いた。
2. 曖昧だった文: 「作業ツリーに未コミットの変更があれば起動前に FAIL」（`.agents/skills/verify-pelway/SKILL.md` の 2 章「ビルドと sha」）。印の nit（`--allow-dirty` の例外が文書に無い）のとおり、ローカルの試し用であること・`allow_dirty` が記録されること・その run は証拠にならないことを明記した。
3. 仮定と解釈: サンプラが TERM で 4 を返す設計は、trap しない限り bash の TERM 既定動作（143）と衝突しない、と読んだ。`wait` が返す他の終了コード（0・3・137・143）は全部失敗に寄せた。
4. 足りなかった情報・検証手段: bwrap の内側で mise/rustup の shim が辿るパスの一覧（試行錯誤で `.rustup`・`.cargo`・`.local/share|state|cache/mise`・`.cache/mbx`・`/mnt/wsl` の bind が必要と判明）。
5. 改善案: `.agents/skills/verify-pelway/SKILL.md` の前提に「共有 git dir の `pelway/` の上の階層が自分か root の持ち物に見えること（見えない環境ではロックの検査で終了コード 2。検査を緩めず、その環境では証拠になる run は取れない）」を足す。

使ったスキル: implement-issue、verify-pelway
## 実装者メモ（round 16）

対象 head: 3a19475d983ef310779be5168e3dd83eaad4aadd

1. 詰まった箇所: Claude の round 10（must-fix 1・印の nit 6・印なし nit 2）の直し: (a) bin ロックの鍵を `target_dir_of`（`CARGO_TARGET_DIR` のみ・`mise exec` の外）で決めていたため、`.cargo/config.toml` の `build.target-dir` や `CARGO_BUILD_TARGET_DIR`・mise の `[env]` で別の出力先を持つ run と別の鍵を引きうる → `cargo_target_dir` で、ビルドに使うのと同じ `mise exec` の中で `cargo metadata --no-deps --format-version 1 --offline` を呼び、`.target_directory` を鍵にする（ビルドの前・ロック獲得の前に一度だけ問う）。(b) `try_lock` が `mkdir` 成功後に pid の書き込みで失敗すると `LOCK` が立たず、ロックの残骸が後続の run を止め続ける → 書き込み失敗時に `rm -f` と `rmdir` してから失敗を返す。
2. 曖昧だった文: 「ビルド先（`${CARGO_TARGET_DIR:-<top>/target}` の実体）を鍵にした 2 つめのロック」（`.agents/skills/verify-pelway/SKILL.md` の 1 章「二重起動の確認」）──「ビルド先」が環境変数だけでなく Cargo の設定まで含む実際の出力先を指すのか曖昧で、`CARGO_TARGET_DIR` だけを見る旧実装を生んだ。いまは「Cargo 自身が報告する `target_directory`」と明記した。
3. 仮定と解釈: `cargo metadata` が必ず解決済みの出力先を返す、と読んだ（`--offline`・`--no-deps` でも `.target_directory` は設定解決の結果で、ロックの鍵として安定）。終了コード 2 の説明は「引数の間違いに加え、起動できない run（証拠・ロックの置き場、git の確認）」を含む、とレビュアーの指摘を解釈した。
4. 足りなかった情報・検証手段: 実物の `start_sampler`/`stop_sampler` の組を試す試験が無く（偽物のプロセスだけで試していた）、trap の終了コードと判定のずれを検出できなかった → `observe_sockets` を no-op にし、使い捨ての `DS_IN` を与えて実物の組を試す試験を足した（`OBS_FAILED=0` を要求）。
5. 改善案: `.agents/skills/verify-pelway/SKILL.md` の 1 章「二重起動の確認」に「ビルド先は、ビルドと同じ `mise exec` の下で `cargo metadata` が報告する `target_directory`」と足す（「ビルド先」の範囲を環境変数の解釈に頼らず Cargo の答えで定義する。今回追加済み）。あわせて冒頭の終了コードの説明を 2 章と同じ定義（引数の誤りに加え、run を始められない失敗）に揃える。

使ったスキル: implement-issue、verify-pelway
## 実装者メモ（round 17）

対象 head: b27857749d87695587c4669cd663c1840f0a2e2f

1. 詰まった箇所: 遅れて戻ってきた Codex の round 10（旧 head 520e6ec5 向け、must-fix 3・印なし nit 3）のうち残り 2 件の直し（3 件目の「ロックの鍵と Cargo の実際の出力先」は round 16 の `cargo_target_dir` で対応済み）: (a) 定期観察が `ps` の確認と `kill` の間にほかの SIGTERM で終わると、`wait` の終了コード 4 だけではこの run の要求か区別できない → 停止の要求は、この run が置く印ファイル（証拠の中の `.sampler-stop`）があるときだけ trap が 4 で応える（印の無い TERM には 5）。stop_sampler は印を置いてから TERM を送り、印を置けない・送れない（確認の後・送信の前に消えたを含む）ときは終了コードに関わらず OBS_FAILED にする。(b) `locks_dir_ready` が検査の前に `mkdir -p` で書き込んでいたため、既存の段がリンクならその先にディレクトリを作りうる → 先に `dir_chain_safe` で既存の段を全部検査し、足りない段はリンクでないことを確かめながら上から一段ずつ `mkdir` し、作った後にもう一度検査する（`mkdir -p` はリンクをたどるので使わない）。
2. 曖昧だった文: 「この run が送った SIGTERM による停止以外」（`.agents/skills/verify-pelway/SKILL.md` の 4 章「失敗」）──「この run が送った」を TERM の到達だけで判断できず、確認と送信の間の終了が抜けていた。印ファイルで「この run の要求」を定義し直した。
3. 仮定と解釈: 印ファイルは証拠のディレクトリ（700）の中なので、この run と同じ権限を持つ他者が置く分には区別できないが、求められているのは誤配・競合の区別であり、同一権限の攻撃者はロックの pid と同じく脅威モデル外と読んだ。印を置く失敗時も TERM は試す（置けないと 5 で応えるので、置けない・届かないのどちらでも OBS_FAILED に届く）。
4. 足りなかった情報・検証手段: 「確認の後・送信の前に消える」競合は実時間では再現できないため、kill がシグナルを送ってから失敗を返す偽物で「送信失敗が合否に届く」ことを確かめる試験にした（同時に、印の無い TERM に実物が 5 で応えることも実物の組で確かめた）。
5. 改善案: `.agents/skills/verify-pelway/verify-run.test.sh` に、途中の段がリンクでもその先に作らないこと・送信失敗で失敗に寄せること・実物のサンプラが印の有無で 4/5 を応え分けることを確かめる試験を足す（今回追加済み）。

使ったスキル: implement-issue、verify-pelway
## 実装者メモ（round 18）

対象 head: b4ec435c3987670663f92134e708b7a7f84118a1

1. 詰まった箇所: round 11（対象 head 12d2bf29、レビューの間に 3 コミット進んだ）の残り指摘の直し: (a) Codex の新 must-fix: 別の clone は共有 git dir が別なので、同じ Cargo 出力先を共有する run が別々のロックを取って実行ファイルを置き換えうる → ビルド先のロックを共有 git dir から、ビルド先そのものの中（`<target>/.verify-pelway.lock`）へ移した（git dir が違っても同じ出力先なら必ず同じロック）。あわせてビルド先の階層を `ensure_safe_dir` で「検査 → 一段ずつ作成 → 再検査」し、ビルド先自身が自分の持ち物でグループ・ほかが書けないことを要求する。(b) 両レビュアーの must-fix: `cargo_target_dir` の試験が呼び出し元の環境（CI の `CARGO_TARGET_DIR=target-ci`）を拾って落ちた → 各ケースで変数を明示的に unset・設定する（5441fd4e で先行対応済み、今回の head でも確認）。(c) nit: 冒頭の終了コードの説明を SKILL.md と同じ定義に、`lock_owner` はファイル全体が数字だけのときだけ、`network.txt` 集計行の書き込み失敗を合否に反映、ロックを外せたときだけ `released lock` を記録（外せなければ NG で FAIL）、demo-check に渡す環境の一覧に `LC_ALL` を追記、判断待ちの「最大 10 秒」を「10 秒の期限まで 1 秒ごと（期限の判定は要求の合間）」に、実装者メモ round 14・16 の引用を実在の文に、round 16 の 5 を提案の形に。
2. 曖昧だった文: 「同じビルド先を共有する別の作業ツリーの run が…置き換えるのを防ぐ」（`.agents/skills/verify-pelway/SKILL.md` の 1 章「二重起動の確認」）──「同じビルド先を共有する別の run」の範囲が「共有 git dir が同じ作業ツリー」までなのか「別の clone」まで含むのか曖昧で、共有 git dir を置き場にする実装を生んだ。いまは「ビルド先の中にロックを置く（clone ごとに別の git dir でも同じロック）」と明記した。
3. 仮定と解釈: ロックをビルド先の中に置いても cargo は触らない（未知のドットディレクトリは無視される）と読んだ。ビルド先がグループ・ほかが書ける場合は、ほかの利用者が実行ファイルとロックの両方を差し替えられるので検査で拒否する側に寄せた（実行ファイルと証拠の対応はもともと保てない）。
4. 足りなかった情報・検証手段: 実際に 2 つの clone で同じ `CARGO_TARGET_DIR` を共有する結合試験（ロックがビルド先の中にあることの機械検査と、リンク先に作らない `locks_dir_ready` 系の試験で代えた）。
5. 改善案: `.agents/skills/verify-pelway/SKILL.md` の 1 章「二重起動の確認」に「ビルド先のロックはビルド先の中に置く（clone 間で共有される出力先でも同じロック）」と足す（今回追加済み）。

使ったスキル: implement-issue、verify-pelway
