# Issue #613 の作業ログ

## 実装者メモ（round 1）

対象 head: bf1bb0d11659828ec9b70a37ff0b8f0c84c3e935

1. 詰まった箇所: `scripts/agent-gh` の push 分岐: `PELWAY_AGENT_SKIP_RESIGN=1` の push は汎用の経路に落ちるため、確認を `push && -z SKIP_RESIGN` の分岐の中に置くと素通りする。サブコマンドの判定（`git_sub`）を分岐の前に出し、両方の経路でフックを掛けた。
2. 曖昧だった文: 「確認を外す環境変数・引数が無い」（Issue #613 の受け入れ条件）。pre-push フック方式では `--no-verify` と `-c core.hooksPath=` が事実上「外す引数」になるため、この 2 つ（省略形と `--config-env` を含む）を拒否する形と読んだ。
3. 仮定と解釈: push される ref と sha は引数を解析せず、pre-push フックの stdin から受け取る。フックはリポジトリに置かず、実行時に一時ディレクトリへ作る（新規の実行ファイルは agent-gh-resign で署名できないため）。repo の設定に `alias.x = push` を置くような意図的な回避は対象外とした（Issue の目的は、素の `git commit` による事故の防止）。
4. 足りなかった情報・検証手段: resign 済みのコミットは committer が `GitHub` になる（`git log origin/main --format='%h %an | %cn | %G?'` で確認）。通常は追跡 ref から辿れるので範囲外になるが、追跡 ref が無い状態で再 push すると止まる。これを確かめる試験は無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり」の commit と push の行に「`scripts/agent-gh dev git push` は pelway-dev[bot] 以外の名義のコミットを含む push を止める」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: cd45d57c52b82101814b146dfb42f5011de469be

1. 詰まった箇所: 手順 8: PR #624 が CONFLICTING のため pull_request の verify が作られず、watch の「All 6 checks passed」は CodeQL などだけだった。`pr view --json mergeable` で衝突に気づき、手順 9 の「修正依頼」として origin/main を merge した。
2. 曖昧だった文: 「差分が `scripts/agent-gh` を変えるときはレビューを委託しない。終わりの報告で `ready:` の行を書かずに止まり」（`.agents/skills/implement-issue/SKILL.md` の手順 8）。`state:` の値が書かれていないため、手順 10 の行から `ready:` を除き、`state: working` とした。
3. 仮定と解釈: `scripts/tests/agent-gh.test.sh` の衝突は、#596（gh の `gh event listen` の案内を除く試験）と #613 の試験が、どちらも末尾に独立した節を足したもの。両方を残した（#613 の節を先、#596 の節を後）。`scripts/agent-gh` は自動で merge でき、#596 の `run_gh` と #613 の git の push の分岐は重ならない。
4. 足りなかった情報・検証手段: watch の通知は必須チェックの有無を示さない。`scripts/agent-gh dev pr checks 624 --required` が「No required checks reported」を返したことで、verify が走っていないと分かった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 8 の「チェックがまだ動いていれば」の前に「`mergeable` が `CONFLICTING` なら pull_request のチェックは作られないため、先に手順 9 の修正依頼で main を取り込む」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 5f8fab27701ab1734738ca00a853481f575fdd92

1. 詰まった箇所: `scripts/tests/agent-gh.test.sh` の `no_bypass`: 連結形（`-ccore.hooksPath=/dev/null`）を既存のループに足しても、実装前から通った。git 2.56.1 が連結形を `unknown option` の exit 129 で拒むため、remote は変わらない。agent-gh の拒否（exit 2 と理由の文）を確かめる別の試験 `attached_c` に書き直し、実装前に `exit 129, want 2` の red を確かめた。
2. 曖昧だった文: 「Git は後から渡された設定を優先するため一時フックが無効になり、他の名義のコミットも送れます」（PR #624 の pelway-review のレビュー、must-fix）。手元の git 2.56.1 では連結形が受け付けられず、再現しなかった。git の版に頼らない防御として拒否を足す、と解釈した。
3. 仮定と解釈: 連結形は `-c?*` の分岐で受け、値が `core.hookspath`（大小文字を問わない）で始まるときだけ拒否する。それ以外の連結形は今までどおり git に渡す（git が拒む）。
4. 足りなかった情報・検証手段: `mise run verify` は deny-check の RUSTSEC-2026-0592（url 2.5.8）で exit 1 になり、ほかのタスクも途中で打ち切られた。この差分は Cargo.lock に触れていない。`mise run verify-lint`・`mise run verify-test`（script-test 40 tests ok）・`mise run shellcheck` を個別に実行し、どれも exit 0 だった。url は main 側の PR #677 で上げる（開発リードの指示）。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の security-review の所見に「回避の経路は、手元の git などの版で再現したかを書く（未再現なら、その旨と版を書く）」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: c3c965f501a07c75e88cd4dea6cdd42d7f1f7130

1. 詰まった箇所: PR #624 の CI（head b87d9ea）: static（deny-check の `advisories FAILED`）、osv-scanner、集約の `mise run verify` が失敗した。手元の deny-check の error は RUSTSEC-2026-0592（url 2.5.8）の 1 件だけで、この差分は Cargo.lock に触れていない。開発リードの指示どおり url は上げず、main 側の PR #677 のマージを待った。
2. 曖昧だった文: 「main 側の修正 PR #677 のマージを待って origin/main を merge で取り込み、verify が緑になるのを確かめる」（開発リードの指示、PR #624 の次の一手）。#677 の状態は watch の対象外のため、#624 の次の通知で `pr view 677 --json state` を確かめて MERGED を知った。
3. 仮定と解釈: origin/main の merge は衝突なし。Cargo.lock の url は 2.5.9 になった。この round の差分の実体は merge だけで、`scripts/agent-gh` と試験は round 3 のまま。
4. 足りなかった情報・検証手段: merge 後の `mise run verify` は exit 0（script-test: 46 tests ok）。CI の osv-scanner の検出の一覧はジョブサマリーにあり、`ci_logs` の末尾には出ないため、url 以外の検出かは CI の再実行の結果で確かめる。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「修正依頼」に「main 側の別 PR の修正を待つときは、その PR の状態を、自分の PR の通知のたびに `pr view <番号> --json state` で確かめる」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: 4da48537a6ee6079983caca99bda3f7047a87b0e

1. 詰まった箇所: `scripts/agent-gh` のサブコマンドの判定: 拒否リスト（`-c core.hooksPath`・`--config-env`・連結形）では、`-c include.path=<file>`（file に `hooksPath = /dev/null`）や `-c alias.p=push p` で一時フックを外せた（PR #624 の 2 回目のレビュー、must-fix 2 件）。include.path の経路は、試験 `rejected (include.path)` で実際に push されることを再現した。
2. 曖昧だった文: 「`-c alias.*` を拒否する。または、サブコマンドが alias なら `git config --get alias.<名前>` で展開先を確かめ、push なら同じ確認を掛ける」（PR #624 のレビュー、must-fix 2）。展開先が `!` で始まるシェルの alias なら、push かどうかを判定できない。そのため、展開はせずに alias をすべて拒否する形を選んだ。
3. 仮定と解釈: グローバルオプションは許可リストを空にし、サブコマンドより前の `-` で始まる引数をすべて exit 2 で拒否する。repo 内に `agent-gh <dev|review> git -...` の呼び出しは無い（`grep -rnoE "agent-gh (dev|review) git [^ ]+"` で確認）。一時フックの `-c core.hooksPath` は `git_cfg` の最後に置く。呼び出し元はグローバルオプションを渡せないので、サブコマンドの直前に当たる。repo・利用者・環境変数の設定にある alias は `git config --get alias.<サブコマンド>` で拒否する（git の alias は組み込みを上書きできない）。pre-push フックを通らない `send-pack`・`http-push`・`remote-*` も拒否した。
4. 足りなかった情報・検証手段: `--attr-source <tree>` のような値を取るグローバルオプションの取りこぼしは、グローバルオプションの全拒否で起きなくなった。`git remote-https` を App の認証で直接使えるかは確かめていない（`remote-*` の拒否で塞いだ）。`mise run verify` exit 0（script-test: 46 tests ok）。
5. 改善案: `docs/agents/` か `scripts/agent-gh` の冒頭のコメントに「agent-gh の git の入口は、グローバルオプションも alias も受け付けない。サブコマンドを最初の引数に書く」と、呼び出し側の決まりとして書く。

使ったスキル: implement-issue

## 実装者メモ（round 6）

対象 head: b7e41321cc123e8e94e323497c95dbc56ea02c0d

1. 詰まった箇所: `scripts/agent-gh` のサブコマンドの判定: round 5 の alias の照会（`git config --get alias.<名前>`）と配管コマンドの拒否リストを、push 後の自動のセキュリティレビューが 2 件指摘した（subsection 形の alias による照会のすり抜けと、rebase -x などの入れ子の push）。試験では、`help.autocorrect=immediate` と打ち間違えた名前（`pushh`）で、実際に push されることを再現した。
2. 曖昧だった文: 「確認を外す環境変数・引数が無い」（Issue #613 の受け入れ条件）。agent-gh は GH_TOKEN を付けて git を起動するので、git が起動する別プロセス（設定の diff・merge のドライバなど）を経由する経路は、クライアント側では閉じられない。この条件の保証の範囲を「agent-gh に渡す引数と環境変数の形」と読んだ。
3. 仮定と解釈: サブコマンドは許可リスト（`git_allowed_subcommands`）と完全に一致するものだけを通す。許可リストは、repo 内で使われている add・commit・fetch・ls-remote・merge・merge-base・push・rev-parse・switch に、確認と取り込みに使う読み取り・作業ツリーの操作を足したもの。コマンドを実行するもの（rebase・bisect・submodule など）は入れていない。alias の照会と配管コマンドの拒否リストは不要になったので消した。
4. 足りなかった情報・検証手段: 環境変数（`GIT_CONFIG_COUNT`・`GIT_CONFIG_PARAMETERS`）と repo の設定で core.hooksPath・include.path を渡す経路は、レビューの文書上の判断だけで、実行して確かめた者がいなかった。試験 `env_block` を足し、どの経路でもフックの確認で止まることを確かめた。`mise run verify` は exit 0（script-test: 46 tests ok）。
5. 改善案: Issue #613 の範囲の外として、`docs/agents/` に「push の名義の完全な強制は、サーバー側（ruleset か必須チェック）で行う。agent-gh の確認は事故の防止」と書く Issue を起こす。

使ったスキル: implement-issue

## 実装者メモ（round 7）

対象 head: a40d9f8d8ff0a1300e29ceeb4144eca46b3d2919

1. 詰まった箇所: round 6 の push の後、自動のセキュリティレビューが 3 件を指摘した。許可リストの照合の glob 化、転送のオプション（`--upload-pack`・`--receive-pack`・`--exec`）によるコマンドの実行、ls-remote の `-u`。転送のオプションは、試験でローカルの remote に対して実際にコマンドが実行されることを確かめた。
2. 曖昧だった文: 「Compare literally rather than as a glob」（自動のセキュリティレビュー、所見 1）。照合 `*" $git_sub "*` の展開はダブルクォートの内側なので、文字列として照合される。`pu?h`・`p*` の試験が、変更前のコードでも exit 2 で通ることで確かめた。誤検知と判断し、試験は回帰試験として残した。
3. 仮定と解釈: 転送のオプションは、全サブコマンドで 3 文字以上の前方一致で拒否する（git の省略形を含む。曖昧な省略形は git でも通らないので、拒否しても失うものは無い）。ls-remote の `-u` は git 2.56 では unknown switch だが、連結形の `-c` と同じく、git の版に頼らない形で拒否した。
4. 足りなかった情報・検証手段: repo の設定の `remote.<name>.uploadpack` や diff・merge のドライバのように、git が設定から起動する別プロセスの経路は試験していない。PR 本文の「対象外」に保証の範囲として書いた。`mise run verify` は exit 0（script-test: 46 tests ok）。
5. 改善案: `scripts/agent-gh` の冒頭のコメントに「git の入口の保証の範囲は、agent-gh に渡す引数と GIT_CONFIG_* の形まで。git が設定から起動するプロセスは対象外」と書き、レビューが範囲外の経路を must-fix にしないで済むようにする。

使ったスキル: implement-issue

## 実装者メモ（round 8）

対象 head: 73aad7dbe112d0bccf98ff2797e9d418ba834938

1. 詰まった箇所: 手順 9 の 4: これは 3 回目のレビュー結果（head a0fa847、must-fix 1 件）で、skill の決まりでは修正に入らずにエスカレーションする場面だった。指揮役の [conductor-wake] コメント（2026-11-24T03:52Z）と開発リードの依頼が、どちらも修正の round に進むよう明示していたため、その判断に従った。must-fix の経路（`GIT_CONFIG_*` の subsection 形の alias `alias.pull push.command=push` と、引数 `'pull push'`）は、試験で実際に push されることを再現してから直した。
2. 曖昧だった文: 「3 回目のレビュー結果に must-fix が残っていたら、4 回目の修正には入らず、止まる条件の形でエスカレーションする」（`.agents/skills/implement-issue/SKILL.md` の手順 9）。指揮役がすでに修正の継続を指示しているときにも、この決まりが掛かるのかが書かれていない。
3. 仮定と解釈: 許可リストの照合は、`qa_check_args` と同じく要素ごとの `==` で完全一致させる。nit は 3 件とも直した。追跡 ref の nit は help と PR 本文の「保証の範囲」に書き、`[文書のずれ]` は docs/dev-environment.md の「レビュー担当の Claude」の表の文を、許可リストの挙動に合わせて書き換えた。
4. 足りなかった情報・検証手段: レビューが未確認とした、設定のフック（`hook.<名前>.*`。hk が入れる形）が pre-push にある repo でも一時フックが走るかは、試験 `env_block (config-hook)` で確かめた。`mise run verify` は exit 0（script-test: 46 tests ok）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の 4 に「指揮役が Issue か PR のコメントで修正の継続を明示したときは、その round を進め、経緯を実装者メモに書く」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 9）

対象 head: 37d713bfdf6840f43c1358da3b7ba823f73d389c

1. 詰まった箇所: codex のレビュー（head 3e66694）の must-fix: 転送用オプションの検査が全サブコマンドの位置引数に掛かり、`git add -- --exec` のようなパスの指定まで拒否していた。検査を、このオプションを解釈するサブコマンド（fetch・pull・push・ls-remote）に絞った。
2. 曖昧だった文: 「転送用オプションを解釈するサブコマンドのオプション位置に検査を絞り、拒否試験に加えてパスの指定が通る試験を足してください。push の検査も同じように全引数を走査するため、オプションの終端を尊重できるか併せて確認してください」（PR #624 の codex レビュー、must-fix）。
3. 仮定と解釈: 「オプションの終端を尊重」は安全にはできないと判断した。値を別の引数で取るオプションは `--` を値として取り込む（実測: `git commit -m -- --no-verify` がメッセージ `--` のコミットを作る）。`git push -o -- --no-verify` は `--` の後ろでも `--no-verify` が効くため、走査を `--` で止めると抜け道になる。位置引数は fetch・pull・push ではリポジトリ名と refspec、ls-remote ではリポジトリ名と参照パターン（最初の位置引数の後ろは、`-` で始まる引数も含めてパターンとして読まれる）。round 9 では全引数を走査する形にしたが、round 10 でオプションの位置だけを走査する形に直した（値の消費と ls-remote の参照パターンを考慮）。
4. 足りなかった情報・検証手段: このスレッドの sandbox では worktree への書き込みが拒否される。`mise run verify` は `target/`（mbx キャッシュ `~/.cache/mbx` への symlink）への書き込みで止まったため、`CARGO_TARGET_DIR=/tmp/target-613` で再実行した。残った失敗は 2 件とも環境起因で、変更を退避した再現で切り分けた: `.agents/skills/verify-pelway/verify-run.test.sh` は `/tmp` の所有者が uid 65534（root ではない）で `dir_chain_safe` が拒否し、origin/main の一時 worktree（`scripts/red-check`）でも同じ 6 件が落ちた。`pelway-client-ui --test no_body_persist` の 2 件は、試験が worktree 内に作る陽性対照ディレクトリの作成が Read-only file system で失敗したもので、origin/main の一時 worktree では 8 件すべてが通った。commit の hook（hk）は、未ステージの変更があると作業ツリーへの stash で失敗するため、メモの追記は実装コミットの後に分けて書いた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「worktree への書き込みが sandbox で拒否される環境では、`CARGO_TARGET_DIR` を書き込める場所に向けて verify を再実行し、書き込みを要する試験の失敗は一時 worktree（`scripts/red-check`）での再現で切り分ける」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 10）

対象 head: 3ade3ecf73a22ad37e71289b2ae82f5c653a9b0d

1. 詰まった箇所: codex の r2 レビュー（head debd0ad9）の must-fix: 転送系サブコマンドの全引数走査が、別の引数を値として取るオプションの値（`git push -o --exec` の --exec は -o の値）と、ls-remote の参照パターン（`git ls-remote origin --exec`）も拒否していた。`--` も値になりうる（`git push -o --`）ため、「`--` で止める」だけでは直せない。
2. 曖昧だった文: 「値を消費するオプションと各サブコマンドの位置引数を考慮して、実際にオプションとして解釈されるものを検査してください」（PR #624 の codex r2 レビュー、must-fix）。
3. 仮定と解釈: git のオプション解釈を一次情報で確かめてから決めた。別の引数を値として取るオプションは、各サブコマンドの `git <sub> -h` で <値> の形のものと builtin の OPT_STRING などから洗い出した（pull の -j・push の --signed など [=値] の形は次の引数を消費しない）。ls-remote は builtin/ls-remote.c が PARSE_OPT_STOP_AT_NON_OPTION で parse_options を呼び、--exec が --upload-pack の隠れた別名であることを source で確かめた（`git ls-remote <repo> --exec` がパターンになることは実測でも確かめた）。fetch・pull・push は parse-options の既定（位置引数の後ろでもオプションを解釈する）を実測で確かめた（`git fetch <repo> --upload-pack <path>` は <path> を実行する）。`--` はオプションの位置に置かれたときだけ解釈を終えるため、値として消費された `--` は終端にしない。
4. 足りなかった情報・検証手段: ls-remote の -u は git 2.56 では unknown switch で実在しないが、版に頼らずオプションの位置では拒否を維持した（既存試験もその形）。`mise run verify` はこのスレッドの sandbox では round 9 と同じ環境起因で実行できない（worktree への書き込み拒否・/tmp の所有者）。`bash scripts/tests/agent-gh.test.sh` は ok。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 の verify の段に「git の引数解釈に関わる検査を実装したときは、`git <sub> -h`・builtin のオプション定義・parse-options のフラグを一次情報として残し、実測で確かめた挙動も書く」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 11）

対象 head: 0e1646f5f232279a50e97a1deb9e398e4bbe46df

1. 詰まった箇所: 指揮役の補足で、codex r2 の must-fix への対応を絞るよう指示があった。claude が debd0ad9 を approve しつつ「git の引数解析を再実装してはいけない」と codex must-fix に反論していたため、両者を踏まえ、検査が扱うオプションの「値を消費する位置」だけを教える形に直した。round 10 で実装したサブコマンドごとの値オプション表（val_longs・val_shorts）は取りやめ、一律走査に戻したうえで `-o`（--push-option・--server-option）の次の引数・オプション位置の `--` の後ろ・ls-remote の最初の位置引数の後ろの参照パターンだけを検査しない形にした。
2. 曖昧だった文: 「検査が扱うオプションの『値を消費する位置』だけを正確に教える限定的な修正に留める」（指揮役の [指揮役・補足] コメント）。
3. 仮定と解釈: 「値を消費する位置」は、must-fix で挙がった -o 系（push の --push-option・fetch・pull・ls-remote の --server-option）の次の引数と、オプションの位置の `--` の後ろの位置引数と解釈した。-o の省略形（--push-opt など）や、ほかの値を取るオプション（fetch の --jobs・pull の -s など）の値は区別せず一律に走査する。git はそれらを値として読むが、誤って値と見なした側の誤りは拒否側に倒れ、安全側には倒れない。`--` はオプションの位置に置かれたときだけ解釈を終え、-o が値として消費した `--` は終端にしない（`git push -o -- --no-verify` は git 側でも --no-verify が効く）。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9・10 と同じ sandbox 起因で実行できない。
5. 改善案: レビュー担当どうしの判定が割れたとき（approve と request-changes の混在）、指揮役が両方の立場を踏まえた修正方針を出す流れがある。その方針の文面を必ず Issue か PR のコメントに残す（実装者が判断の根拠を後から追えるように）。implement-issue の「指揮役・人間の決定は受け入れ条件と同じ重さ」の運用と整合する。

使ったスキル: implement-issue

## 実装者メモ（round 12）

対象 head: 4a97fc8adf0e29a64c805b4fdef9591fc40a894b

1. 詰まった箇所: claude r3 の must-fix（round 11 で足した 3 つのマーカー例外は、マーカーの引数が別のオプションの値として消費される形でも発火する）を直した。指摘の 6 形を本物の git で実測してから直した: `push --repo -- --no-verify`（-- が --repo の値で --no-verify が効く）、`push -vo -- --no-verify`（まとめた短いオプションの末尾の o が -- を消費）、`push --repo -o --no-verify`（--repo が -o を消費）、`fetch -vo -- --upload-pack=<cmd>`・`ls-remote --sort refname --upload-pack=<cmd>`（いずれも <cmd> が実際に動く。`--sort` の値 refname が位置引数に見えて ls-remote のパターン扱いで走査が止まる）。
2. 曖昧だった文: レビュアーの修正案「マーカーを尊重するのは、それより前の全引数が『-o 系の綴り+その値』または『= 付き long option』のときだけ」。
3. 仮定と解釈: clean な前の引数（-o 系の綴りとその値・`=` 付きの長いオプション。どちらも綴りから次の引数を消費しないと分かる）だけが先にあるときは、マーカーは必ず正しい（-o の次の引数は値、`--` の後ろは位置引数、ls-remote の最初の位置引数の後ろは参照パターン）。それ以外の綴り（`-` 始まりの別の形・位置引数）が一度でも来たら clean は崩れ、以後は末尾まで一律に走査する（round 9 の全引数走査と同じ）。round 11 まで通していた `fetch origin -- --upload-pack=...`・`push origin -- --exec` は、位置引数で clean が崩れるため拒否側に戻した（git では位置引数扱いだが、値に埋まった `--` との区別が付かないため意図的な過剰拒否）。
   あわせて nit 対応: pre-push フックの `git log` に `--no-replace-objects`（git 本体のオプション。`git log` の引数ではない）、`core.hooksPath` を `--type=path` で読む、許可リストの転送系以外の名前が `git <名前> -h` に転送オプションを持たない試験を追加、試験のコメントからレビューの経緯を外した。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜11 と同じ sandbox 起因で実行できない。レビュアーの「『値と誤読しても拒否側に倒れる』は『マーカーと誤読すると許可側に倒れる』点で成立しない」が的確だった。round 11 で例外を足すとき、例外が発火する条件が git の解釈とずれるケース（値に埋まったマーカー）を試験していなかった。検査を外す例外には、その例外自身が安全側に倒れる条件を付けるべきだった。
5. 改善案: 「一律走査から例外を引く」形の検査を書くとき、例外の発火条件が「それより前の全引数が、次の引数を消費しないと綴りから分かるもの」でなければ fail-open になる。git のオプションは値を別の引数で取れるため、先に出た綴りを値として消費する可能性は常にある。この形の検査を設計するときは、例外の適用条件を先に書いてから実装する。

使ったスキル: implement-issue

## 実装者メモ（round 13）

対象 head: 792055b834ae1be1506ade7cfc820b802e964849

1. 詰まった箇所: claude r4 の must-fix。round 12 で「clean な `--` の後ろは位置引数」とした例外が、pull・push では成立しない。両者は `--` を外して後ろの引数を子の git に渡す（pull は builtin/pull.c の run_fetch が `git fetch` を起動、push は submodule.c がサブモジュールの `git push` を起動）ため、子はそれをオプションとして解釈する。`git pull -- . --upload-pack=/bin/echo` を実測し、/bin/echo が動くこと（出力が protocol error に出る）を確かめた。修正方針どおり、`--` で走査を止めるのは fetch・ls-remote だけにし、pull・push では `--` で clean を崩して末尾まで走査する。
2. 曖昧だった文: 「`--` で走査を止めてよいのは fetch・ls-remote のみ」（レビュアーの修正方針）。
3. 仮定と解釈: fetch・ls-remote は `--` の後ろを子プロセスに再解釈させる経路がなく、位置引数としてだけ読むため、clean な `--` は終端として安全。pull・push は既存試験の `push -- --receive-pack=...`・`push -o x -- --exec` を拒否に反転し、`pull -- . --upload-pack=...`・`pull -o x -- --receive-pack=...`・`push --recurse-submodules=on-demand -- origin --no-verify`（サブモジュール push のフックを外す形）を拒否試験に足した。fetch・ls-remote の `--` は通す側に残し、`fetch -- --upload-pack=...`・`ls-remote -o x -- --exec` を通過試験に足した。レビュアー注記の out-of-scope（PATH 差替えで名称検査ごと迂回される問題。main も同じで本当の直しはサーバー側 ruleset）は報告のみで、PR 本文には書かない。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜12 と同じ sandbox 起因で実行できない。round 12 で付けた clean 条件は「前の引数が次の引数を消費しない」ことだけを見ていたが、`--` の後ろの引数が「この git」でなく「子の git」で解釈される経路を見落としていた。ラップされる側のコマンドが子プロセスに引数を引き継ぐかは、同じ git のサブコマンドでも違う（pull・push は引き継ぐ、fetch・ls-remote は引き継がない）。
5. 改善案: 検査対象のコマンドが別プロセスを起動するとき、wrapper が付け外しした区切り（`--` など）が子にどう見えるかは、親の解釈と別に確かめる。git のようにサブコマンドが子の git を起動する形では、`--` の終端としての効力は親と子で一致しない。サブコマンドごとに「後ろの引数が子プロセスに渡るか」を調べる一覧があると、同じ形の検査を設計するときに役立つ。

使ったスキル: implement-issue

## 実装者メモ（round 14）

対象 head: 2a6b195043f735b0205206eebb881e919453fb6a

1. 詰まった箇所: codex の must-fix。既存の pre-push を呼ぶ処理が `git config --show-scope` の出力を awk のタブ区切り `$2` で読んでおり、タブ・改行を含む `core.hooksPath` のパスが壊れる（名義確認を通った push で元フックの拒否をすり抜ける）。`-z` の出力が `scope\0value\0` の並びであることを od で確かめ、NUL 区切りの read 対に書き換えた。`--type=path`（round 12 で追加）はそのまま残した。
2. 曖昧だった文: 「NUL 区切りでスコープと値を読み」（must-fix 本文）。
3. 仮定と解釈: `git config -z --show-scope` は entry ごとに scope と value を別々の NUL 終端フィールドとして出す（実測）。元の awk は `END {print v}` で最後の非 command 値を取っていたため、ループでも最後の非 command 値を保持する。試験はタブを含むパスに拒否を返す pre-push を置き、フックの呼び出し（marker ファイル）・非 0 終了・remote の ref 不変を確かめる。設定値に改行は .git/config には書けないが、NUL 区切りはどちらにも対応する。あわせて nit 2 件: ls-remote の説明を「最初の位置引数をリポジトリ名、その後ろを参照パターン」に修正、author だけが異なるコミットを止める試験（author_only_block）を追加。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜13 と同じ sandbox 起因で実行できない。round 12 で `--type=path` を足すとき、出力の区切り文字と値の中の文字の衝突（タブ区切りの出力にタブ入りの値）は見ていなかった。機械が出す区切りのある出力を awk のフィールドで読むときは、値が区切りを含みうるかを常に確かめる。
5. 改善案: `git config` の出力をプログラムから読むときは最初から `-z`（NUL 区切り）を使う習慣にする。`--show-scope`・`--show-origin`・`--name-only` など表示系の区切りはどれも値の中の文字と衝突しうる。

使ったスキル: implement-issue

## 実装者メモ（round 15）

対象 head: d5219a72ef4564e3563881c8420fb303e742bb28

1. 詰まった箇所: codex の must-fix。round 14 で NUL 区切りに変えたとき、`git config` をプロセス置換 `< <(...)` に移したため、読み取り自体の失敗（終了コード 1 以外）が while に伝わらず、orig が空のまま既定のフックディレクトリに進む fail-open になっていた（以前は pipefail で失敗がフックの失敗になっていた）。読み取りを別の呼び出しで先に試し、終了 1（キーが無い）だけを許してそれ以外は push を止める形にした。
2. 曖昧だった文: 「読み取り元の終了ステータスを確認し、設定を取得できなければ push を止めてください」。
3. 仮定と解釈: 「終了ステータスを確認」は 0 と 1（キーが無い）を区別する、と解釈した。`git config --get-all` は値が無いとき 1 を返す。NUL を含む出力は bash の変数・コマンド置換に入れられないため、失敗の確認だけを `>/dev/null` の呼び出しで行い、値の読み取りは別の `git config` をプロセス置換で読む形にした（同じコマンドを 2 回呼ぶ。間で設定が変わる可能性は実用上ない）。試験は `~<存在しないユーザ>` を含む hooksPath で、フック内の `git log` は agent-gh の -c（command スコープ）で動くが全値を展開する `--get-all --type=path` だけが失敗することを実測で確かめ、読み取り失敗で止まり remote の ref が変わらないことを試験した。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜14 と同じ sandbox 起因で実行できない。round 14 で「パイプの失敗が伝わる」を「プロセス置換の失敗が伝わらない」に置き換えたことで、エラー経路の意味を変えていた。読み取り系の書き換えでは、失敗したときの出口（エラーとして止まるか・既定値に進むか）を元のコードと見比べるべきだった。
5. 改善案: パイプやプロセス置換の形を変えるリファクタリングでは、プロデューサーの終了ステータスの扱い（pipefail で伝わる・伝わらない・捨てる）を diff の一部として明示的に確認する。bash の `while read` ループは、パイプでは subshell、プロセス置換では親の変数を使えるが失敗が伝わらない、という形ごとの制約を図にしておくと抜け道を作りにくい。

使ったスキル: implement-issue

## 実装者メモ（round 16）

対象 head: 84d7d386837b6f5a364ee5c4dc13e6c2cbd1a634

1. 詰まった箇所: codex の must-fix（投稿保留のレビュー経由）。round 15 で成否の確認（>/dev/null の呼び出し）と値の読み取り（プロセス置換）を別の git config に分けたため、実際に値を読む側の呼び出しが失敗しても拾えない fail-open の別経路が残っていた。途中、終了コードを流れの末尾に `@@cfg-rc` レコードとして乗せる形を試したが、`--get-all --type=path` が値の展開に失敗するときスコープのフィールドだけを出してレコードの途中で切れるため、scope/value の対がずれて印が値として飲まれることを試験で再現してから止めた（push が実際に通った）。値と終了コードを同じ呼び出しから得るため、一時ファイルに書く形にした（trap で掃除）。
2. 曖昧だった文: 「設定値と終了コードを同じ呼び出しから取得する形に」。
3. 仮定と解釈: NUL を含む出力は bash の変数・コマンド置換に入れられない、プロセス置換は終了コードを返さない、パイプは subshell で親の変数を使えない、の 3 制約を同時に満たす形として一時ファイルを選んだ。試験は round 15 の `cfg_read_fail`（`~<存在しないユーザ>` の hooksPath）がそのまま有効で、印を流れに乗せる形では red（push が通る）になることも確かめた。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜15 と同じ sandbox 起因で実行できない。「印をストリームに混ぜる」工夫は、プロデューサーが構造化レコードの途中で死ぬと対がずれて印自体が飲まれる。印を信頼できるのは、プロデューサーが常に整ったレコードを出すと分かるときだけ。
5. 改善案: 「値を読む呼び出し」と「失敗を確かめる呼び出し」を分けたコードは、2 回呼ぶ前提自体を疑う。bash で NUL 区切りの出力と終了コードの両方が要るときは、素直に一時ファイル（または coproc + wait）を使う。ストリームに印を混ぜる形は、プロデューサーがレコードの途中で失敗しないことが保証できない限り選ばない。

使ったスキル: implement-issue

## 実装者メモ（round 17）

対象 head: 131ffc84e4f52166d407d4914b2859f96e163bbc

1. 詰まった箇所: codex r12 の must-fix 2 件。core.hooksPath の照会（`git config -z --show-scope --get-all --type=path`）の終了コード 1 を「キー無し＝既定の場所」と読んでいたが、照会先を GIT_CONFIG で別ファイルに固定された環境では、`git config` はそのファイルだけを読み、agent-gh が -c で付けた一時設定は照会に出ない（実測）。終了コード 0 かつ command スコープの値として一時フックの dir が取れることを必須にした。もう 1 件は TMPDIR が相対パスのとき、mktemp が呼び出しの cwd 基準の相対パスを返し、git が core.hooksPath の相対パスを worktree ルート基準で解決するため、サブディレクトリ呼び出しで一時フックを見つけられず push が素通りする。存在しない core.hooksPath で push が失敗しないこと（フックなし扱い）を実測で確かめ、`cd && pwd -P` で絶対パスに解決してから渡す形にした。
2. 曖昧だった文: 「照会には :458 の一時フック設定が command スコープ値の取得確認を必須化し、照会失敗は fail-closed に」。
3. 仮定と解釈: 「command スコープ値の取得確認」は、照会結果の command スコープの値に一時フックの dir（絶対パス）自体があること、と解釈した。GIT_CONFIG で照会先を固定された環境では照会の command スコープの値がそのファイルの値になるため、dir との一致で確かめる。claude の nit「`2>/dev/null` で git のエラー文が消える」は、指揮役の指示（comment 7138751822）が「印 nit: claude approve@84b0072a の nit 2 件」とするため対象に含め、照会失敗時に stderr の文を添えて出す形にした。照会の一時ファイルは一時フックの dir の中に置き、TMPDIR が相対・存在しない dir を指していても作れる場所にした（mktemp をやめた）。
4. 足りなかった情報・検証手段: `bash scripts/tests/agent-gh.test.sh` ok。`mise run verify` は round 9〜16 と同じ sandbox 起因で完走できない（`CARGO_TARGET_DIR=/tmp/target-613` を指しても ld が bus error・verify-run.test.sh が run dir を作れない・hk-check の typos が main 由来の worklog で落ちる）。回帰試験は `scripts/red-check 84b0072a --keep scripts/tests/agent-gh.test.sh -- bash scripts/tests/agent-gh.test.sh` で旧実装に red（cfg_env_override・rel_tmpdir_block で remote の ref が変わる）になることを確かめた。GIT_CONFIG で照会先を固定した環境での `git config` の挙動（ファイルの値が command スコープとして返り、-c の値は照会に出ない）は実機で確かめた。
5. 改善案: 「キー無し」を許す照会では、照会が意図した読み取り先を見ているかの検証手段が要る。照会で必ず返る値（自分で付けた -c の値など）を結果に含めることで、「値が無い」と「照会が別のものを見ている」を区別できる。

使ったスキル: implement-issue

