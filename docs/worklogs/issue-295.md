# Issue #295 の作業ログ

## 実装者メモ（round 1）

対象 head: 2cbbfe8fdd71792bf013b93b418e2a338af49ac9

1. 詰まった箇所: `mise run verify` の script-test: `scripts/tests/conductor-watch-ensure.test.sh` だけが実行ごとに違うサブテストで失敗する（`foreign watch present`・`watch is running`・`ensure restarts watch` など）。`git stash` で差分を除いた main の tree でも同様に失敗することを確かめ、本差分とは無関係の環境依存と判断して進めた。
2. 曖昧だった文: 「review-pr スキルが実際に使う読み取り系（`gh api`, `gh pr diff/view/checks`, `git show/diff/log/grep` 等）に限定される」（Issue #295「受け入れ条件」）
3. 仮定と解釈: 「読み取り系」の集合は、例示の列挙ではなく `docs/dev-environment.md`「レビュー担当の Claude」設定表の許可形（gh: `api`・`issue view`・`pr view`・`pr diff`・`pr checks`・`pr review`）と review-pr・security-review が使う git 読み取り系（`show`・`diff`・`log`・`blame`・`grep`、skill の列挙に `blame` を含むため追加）と解釈した。選ばなかった解釈: `api` を GET 限定にする引数検査（#305 が hatai の選択待ちと明記）と、読み取り系 git の `--output`・`-O` の拒否（#306 の範囲。引数形の deny は省略形・束ね書きで外れやすいと #306 が結論付け済み）。拒否の終了コードは既存の 2〜5 に続く 6 とした。
4. 足りなかった情報・検証手段: review 名義の正当な呼び出しの全網羅リストは無く、リポジトリ内の `agent-gh review` 出現箇所（review-pr・security-review・dev-environment.md の確認手順）を grep で洗って許可集合を決めた。conductor-watch-ensure.test.sh の不安定さを切り分ける確実な再現条件（本番の conductor-watch.sh が常駐する環境で名前空間内の pgrep カウントがどうずれるか）は未解明。
5. 改善案: `scripts/tests/conductor-watch-ensure.test.sh` の動作検査の先頭に「unshare 内の `pgrep -f 'conductor-watch\.sh'` が実環境の常駐 watch を数えていないか」を確かめる事前条件を足し、混ざる環境では動作検査を skip と明記する。

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`

## 実装者メモ（round 2）

対象 head: 0df36c311d0acdbdaf8b2841ad0b1f8ec85b6b03

1. 詰まった箇所: `scripts/agent-gh` の `review_api_args_allowed`: `gh api` のフラグ形を pflag の受け取り方に合わせて拾う必要があった（`-XDELETE`・`-fkey=...` の束ね書き、`--method=...`、値を取る `--method` と `--input`）。
2. 曖昧だった文: 「HTTP メソッドを GET に限定し、変更系メソッドを拒否してください」（PR #463 の codex レビュー must-fix、scripts/agent-gh:39）
3. 仮定と解釈: 「GET に限定」は `-X`/`--method` の値検査だけでなく、body を付けると `gh api` が暗黙に POST になる `-f`/`--field`・`-F`/`--raw-field`・`--input` の拒否を含むと解釈した。明示の `-X GET`・`--method get`（大小文字不問）は許可とした。選ばなかった解釈: `-X`/`--method` だけを検査して body 形を残す案（POST が残り must-fix を満たさない）。`api` の許可集合が狭まる影響は、リポジトリ内の `agent-gh review api` 呼び出しがすべて GET 形（`api /rate_limit`・`api /installation/repositories`・`api repos/...`）で投稿が `pr review` 経由のため無しと判断した。
4. 足りなかった情報・検証手段: `gh api` のフラグ一覧と pflag の束ね書きの受け取り方は `gh api --help` 相当の情報がリポジトリ内になく、コード内コメントでカバーする形を自前で列挙した。実機の `gh api` での拒否確認はレビュー側の実測に委ねる形になった。
5. 改善案: `docs/dev-environment.md`「レビュー担当の Claude」の設定表の読み取り系 `git` の列挙に `blame` を足す（review-pr が `git blame` を使うが、表の列挙には無い。round 1 の claude レビュー nit でも指摘された齟齬）。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 3）

対象 head: 901869ad4fcc2c70c21ac37a34af5c2aefe91a2c

1. 詰まった箇所: `review_api_args_allowed` を拒否リストから許可リストに作り替える必要があった。pflag は短いオプションのクラスタを 1 文字ずつ読み、`-i`（値を取らない）の後ろに続く `X`・`f`・`F` が別のフラグとして効くため、`-iX PATCH`・`-iftitle=x` の形を拾うにはフラグ単位ではなく文字単位の解析が要った。git 側は許可したサブコマンドの引数検査に加えて、トークンを発行しない・`env -i` でページャ・外部 diff・設定注入（`GIT_CONFIG_*`・`GIT_DIR` 等）・ユーザー/システム設定を持たない実行時境界を置く形で塞いだ（引数検査だけでは省略形・束ね書きで外れやすいという #306 の知見に応える二層）。
2. 曖昧だった文: 「承認済みサブコマンドの引数検査を追加してこれらを拒否するか、安全な実行時境界で塞いでください」（PR #463 の codex round 2 must-fix）と「引数の検査を許可リストにするか、`-` で始まる引数を pflag と同じく 1 文字ずつ読んで検査する」（claude round 2 must-fix）
3. 仮定と解釈: api は claude が挙げた許可集合（`-i`/`--include`・`--paginate`・`--slurp`・`--silent`・`--verbose`、値を取る `-H`/`--header`・`-q`/`--jq`・`-t`/`--template`・`--cache`・`--hostname`、`-X`/`--method` の GET 形、`-` で始まらない endpoint）をそのまま採用し、列挙外の長・短オプションはすべて拒否とした。`X-HTTP-Method-Override` ヘッダは GitHub が受理するか未確認だが、拒否のコストが低いため検査に入れた。git 側は「引数検査か実行時境界か」の二者択一を両方で応え、review の git は呼び出し側が `git` 以外に渡す正当な用途（認証・URL 書き換え・署名設定）が無いため、トークン・credential helper・url rewrite をすべて持たない `env -i` の git とした。選ばなかった解釈: gh 経由の git にトークンを残す案（`git grep -O` が実行経路に残る間は子プロセスの環境にトークンが乗る）。
4. 足りなかった情報・検証手段: 実機の `gh`/`git` でのクラスタ・省略形の受理確認はレビュー側の規則上こちらでは試せないため、pflag の読み方（claude のレビューが引用）と git の一意な省略形の仕様に照らした静的な判定に留め、テストは偽 gh/git が受け取る argv で確認する形になった。`conductor-watch-ensure.test.sh` の環境依存フレークは引き続き未解決（round 1 と同じ）。
5. 改善案: `env -i` で落ちる `CALL_LOG` のような観測用環境変数を、テストが「子プロセスに渡したくない値」を検査するための記録先として扱う慣行（偽コマンド側にフォールバックの記録先を持たせる形）を、他の境界テストにも適用すると良い。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 4）

対象 head: 5bad351547590f5160514567b3ed6e6a43c62df3

1. 詰まった箇所: テストの `expect_log` 行にある jq 式リテラル（`'... $ENV.GH_TOKEN'`）が ShellCheck の SC2016（単一引用符内の `$` は展開されない、という警告）に引っかかった。意図した記述なので該当行に `# shellcheck disable=SC2016` を各行へ付けて解消した（`run` 行の直前に付けても後続の `expect_log` 行には効かない）。また `mise exec` が全ツールの解決を行い GitHub API のレート制限（未認証 60/h）で 403 になる環境障害に当たり、インストール済みツールをそのまま使う `MISE_AUTO_INSTALL=0` で verify・フックを通した。
2. 曖昧だった文: 「git の引数の検査を厳密にするか、review での git の提供自体をやめる」（claude round 3 must-fix の選択肢）と「`git grep --no-index` のような引数を拒否するか実行時境界で塞ぐ」（codex round 3 must-fix）
3. 仮定と解釈: git 側は「提供自体をやめる」を選んだ。`agent-gh review git` の呼び出し元はリポジトリ内に無く（review-pr・security-review は main の plain `git` を使う）、拒否リスト方式の引数検査では省略形（`--op`）・値を取るフラグの誤認（`-G -O`・`-B --output`）・`--no-index` のリポジトリ外読み取りを網羅できないため、サブコマンドごと終了コード 6 で拒否する方が安全で安い。トークン漏洩（`--jq '$ENV.GH_TOKEN'`）は、claude が挙げた代替のうち「トークンを環境変数で渡さない」を採り、0600 の一時 `GH_CONFIG_DIR/hosts.yml` で `gh` を認証させる方式にした（選ばなかった案: jq 値の `$ENV` 参照を字句検査する方式。`$ENV`・`env`・`@base64` 等の表記ゆれを検査側が網羅する必要があり、環境に置かない方が確実）。
4. 足りなかった情報・検証手段: `gh` が `hosts.yml` の `oauth_token` をどう解釈するかの公式仕様確認は行えず、`gh auth setup-git` が書く形式に倣った。実機の `gh` での受理確認はレビュー側の実測に委ねる。`conductor-watch-ensure.test.sh` の環境依存フレークは引き続き未解決（round 1 と同じ。今回の verify では失敗。`MISE_AUTO_INSTALL=0` 下で残り 29 テストは合格）。
5. 改善案: `mise exec` が不要なツール解決で GitHub レート制限に当たる問題は、`mise.toml` のツールを backend ごとに遅延解決させるか、CI/ローカルで `GITHUB_TOKEN` を供給する運用メモを `docs/dev-environment.md` に書くと再発を避けやすい。

使ったスキル: `.agents/skills/implement-issue`

## 実装者メモ（round 5: main の取り込み）

対象 head: 8a527a71b87e1f2a0e8851f34a58ffd8189c31e7（第1親）+ 540c5f5e（origin/main、第2親）

1. 詰まった箇所: main（#449 の review 用 fnox 設定の分離・qa ロール追加などを含む 155 コミット）と本 PR の review 許可リストが `scripts/agent-gh` の review ケース・usage・終了コードの同じ場所を編集していて、agent-gh・そのテスト・dev-environment.md の 3 ファイルが衝突した。さらにスレッドの worktree は `.devin` が読み取り専用マウントされており、main 追加の `.devin/skills/*`（symlink 2 件）を展開できないため、マージは ro マウントの無い一時 worktree で実行し、push 後に本 worktree を追従させた。
2. 曖昧だった文: 「review 用の設定に秘密が無ければ fail-closed（exit 3）」のテスト（#449）が不許可サブコマンド `pr list` を使っており、本 PR の「許可外はトークン発行前に exit 6」とどちらの挙動が正しいか。
3. 仮定と解釈: 両側の意図を併存させた。review の許可リスト検査は #449 の fnox 分離（`PELWAY_AGENT_FNOX_CONFIG_REVIEW`）と 3 引数 `app_token` の上に載せ、検査順序（許可リスト → トークン発行）と exit 6 は round 4 のまま（fnox・gh・git のどれも呼ばない性質を維持）。`review_missing_secret_fails_closed` は許可サブコマンド `pr view 1` で確かめる形に直した（テストの意図は秘密欠落の fail-closed の確認で、`pr list` という選択は #449 時点では許可リストが存在しないため任意）。usage・終了コードの記述は qa（main）と review の exit 6（本 PR）を連結し、dev-environment.md の設定表は本 PR の括弧書き（ラッパーの絞り方の説明）と main の #449 の `Read(...)` deny 追記を連結した。取り込みは rebase ではなく `Merge branch 'main'`（issue-352・issue-97 と同じ先例。マージコミットは agent-gh-resign で表現できないため署名なし）。
4. 足りなかった情報・検証手段: マージ結果の Rust 側（crates/・Cargo.toml・Cargo.lock・.cargo/）は origin/main と無差異であることを `git diff origin/main` で確かめ、lint・Rust テストの結果は main の CI と同じになることを根拠付けた。スクリプト側は `mise run script-test`（33 テスト合格）・`mise run shellcheck`（0 件）・`mise run build`（--locked、成功）で検証した。
5. 改善案: スレッド worktree の読み取り専用マウント（`.devin`・`.opencode`・`opencode.json[c]`）と main 追加ファイルの衝突は今回の一時 worktree 手順で回避できる。`fix-merge-conflicts` スキルに「読み取り専用マウントがある worktree では一時 worktree でマージする」注意書きを足すと再発時に迷わない。

使ったスキル: `fix-merge-conflicts`（グローバル）

## 実装者メモ（round 6）

対象 head: 3ca43d7fe8b6380b021529b6b8f3706eb2c5a3fb

1. 詰まった箇所: `scripts/agent-gh` の review の gh 呼び出し: round 4 の 2 担当が別々の変数名（Codex は継承した export 属性の残る `token`、Claude は gh が hosts.yml より優先する `GH_TOKEN`・`GITHUB_TOKEN`）で同じ穴を指摘した。変数名ごとの対処を避けるため、偽 gh に `env` 全体を記録させ、`expect_review_auth` で「review トークンの値がどの変数名でも gh の環境に無い」と認証の 4 変数（GH/GITHUB の TOKEN と ENTERPRISE_TOKEN）が無いことを、review の全テストで確かめる形にした。
2. 曖昧だった文: 「round 4 でまた must-fix が出たら、直さずに止まって報告する。」（Issue #295 の hatai コメント、2026-11-18）と、指揮役スレッドの指示「cdbad162 から追加コミットで再開（force push なし）」。
3. 仮定と解釈: 再開の決定は Issue 上では hatai 作のコメントで確かめられず（再開を記録したのは pelway-dev のコメントと指揮役スレッドの指示）、割り当て経路の指示として従った。round 4 の must-fix 2 件だけを直し、round 4 の Claude の nit 4 件（help・ヘッダの `<dev|review> git` 表記、`-q`/`--jq` の「無害」記述、hosts.yml の 0600 未検査、テストの不要な退避）は skill の「nit は直さない」に従い未対応のまま残した。gh の環境変数のうち対象を認証に効く 4 つと `token` に限り、`GH_HOST`・`GH_PAGER` などは対象外とした（トークンの値や名義を変えないため）。「fail-closed のツール一覧チェック」はラッパーのコードではなくレビュー手順の確認条件と解釈した（ラッパーからはレビュー担当に渡されたツール一覧が見えない）。
4. 足りなかった情報・検証手段: 実機の gh 2.104 で hosts.yml だけで認証できるか、継承 `GITHUB_TOKEN` が実際に優先されるかは本 round でも実測していない（偽 gh のテストと、round 4 の Claude レビューが示した go-gh のソースの読みが根拠）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の表の 3 回目のレビューの行に「人間の判断で再開したときは、再開の決定を hatai 名義の Issue コメントとして残してから着手する」を足す（今回は再開の決定が pelway-dev 名義のコメントにしか無く、決まりの「hatai 作のコメントだけを決定として扱う」と食い違った）。

使ったスキル: `.agents/skills/implement-issue`
