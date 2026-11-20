# Issue #510 の作業ログ

## 実装者メモ（round 1）

対象 head: 50db8f76561da2581024b4248b2ace42120595e9

1. 詰まった箇所: 手順 6（verify）: clippy `suspicious_open_options` が `create(true)` に truncate の明示を要求し、lock ファイルは中身を使わないので `.truncate(false)` を書いた。手順 4: `unwritable_state_does_not_repeat_events` が state ディレクトリを 0555 にするため lock 作成に失敗し早期終了するようになったので、lock ファイルを先に置く形に直した。
2. 曖昧だった文: 「取れなければ二重起動として、理由を1行ログに出して終了する」（Issue #510 本文「やること」）— 終了コードが書かれていない。`scripts/agent-worktree-prune` の同種処理（`flock -n` 失敗で `exit 0`）に合わせて 0 にした。
3. 仮定と解釈: 「起動時に比べる」は `--once` / `--dry-run` を含む全モードに適用すると解釈した（worktree での起動自体を拒否する趣旨のため）。`git rev-parse` が失敗して起動可否を確かめられないときは、開くより閉じる方が安全と判断し終了コード 1 にした。`--project` の指定先で rev-parse する点は、`t3`/`gh` と同じ「見張る対象の checkout」を見る解釈。
4. 足りなかった情報・検証手段: Issue に拒否時の終了コードとログ文言の指定がなかった。`agent-worktree-prune`（worktree 判定・flock・exit 0）と `main.rs` 既存の `agent-watch: ...` 1 行ログの文体に揃えた。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「プロセスを拒否・終了させる条件では終了コード（意図した終了か失敗か）も書く」を足すと、今回のように終了コードを先行例から推す手間が減る。

使ったスキル: implement-issue。輸入スキルはなし（pelway-agent-watch は `.cargo/mutants.toml` の examine_globs 対象外のため `mutants:diff` 不要と判断）。

## 実装者メモ（round 2）

対象 head: 1eac47ea56bf3b8475ac562355f6d452f9543186

1. 詰まった箇所: 手順 8（レビュー委託）: `codex_review` の委託が 2 回連続 `failed`（子の環境で `scripts/agent-gh review` が `gh-token` 拡張欠如のため GitHub App トークンを発行できない。実装側の環境では `scripts/agent-gh review pr view` は動く）。停止条件に該当するため、round 2 では Claude のみに再委託し、Codex 分は Issue にエスカレーション。結果: round 2 の Claude 委託は 1 回目がチェック未完了で `failed`（再委託は 1 回までの扱いでリトライ可と解釈）、再委託で `approve`（must-fix 0、GitHub 上の APPROVED と照合済み）。PR #521 は Codex レビュー未実施のまま Draft で人間の環境修復待ち。
2. 曖昧だった文: 「Codex・Claude（`claude_review`）・Devin CLI の使えるもの全て（3 人）」（`.agents/skills/command-team/SKILL.md`「レビュー担当と effort」）— 委託先の環境が壊れているときが「使えない」に含まれるかが書かれていない。再委託上限の停止条件が先に効くと解釈し、Codex を外した委託はせずエスカレーションにした。
3. 仮定と解釈: 外部 `flock` 代用のテストを本物バイナリの spawn に替える指摘は妥当（`let _lock`→`let _` で lock が外れる退化を検出できない）と判断し、受け入れ条件の形（「2つ目の起動が拒否される」＝起動中の agent-watch がいる状態）に合わせて直した。nit（dry-run のガード免除・`file.set_permissions`・panic 時の子始末）は指示なしとして直していないが、KillOnDrop は must-fix 側の実装に必要なため導入し、同じ形を外部 flock 版にも使った。
4. 足りなかった情報・検証手段: `codex_review` の子環境に `gh extension install Link-/gh-token` が無いことは委託先の報告でのみ分かり、こちらからは観測・修復できない。事前に子環境を確かめる手段が無い。
5. 改善案: `docs/dev-environment.md`「レビュー担当の Codex（codex_review）」の「レビュー委託（delegate_task）での確認」に「`scripts/agent-gh review` がトークンを発行できる（`gh-token` 拡張が委託先の環境で使える）」の確認行を足す。11-14 の確認合格後に環境が回帰しても、今回のように委託 2 回の失敗でしか気付けないため。

## 実装者メモ（round 3）

対象 head: 60b0b803631e77a18511ba1cfedfc589f4e3fdd3

1. 詰まった箇所: `be9be0c` で CI の `test (cargo test)` が落ちた（`externally_held_lock_is_refused` の `wait_locked` が "lock was never taken" で panic）。原因はテスト側の競合: `wait_locked` の検知が `flock -n <lock> true` ポーリングで、観測自体が一瞬 lock を取る。ホルダーの `flock -n <lock> sleep 30` の 1 回きりの試行と重なるとホルダーが終了する。`/proc/locks` を読むだけの検知（lock を取らない観測）に変更し、保持側が早期終了したときは `try_wait` で即検出する形にした。`second_instance_is_refused`（本物バイナリがホルダー）も同じ競合窓があるため同じ修正。
2. 曖昧だった文: なし（skill の「Approve の後は PR に push しない」に round 2 で抵触して head を動かし APPROVED を無効化してしまったのは、こちらの手順ミス。レビュー委託前の修正 round としては正しい流れ）
3. 仮定と解釈: `wait_locked` の呼び出し側は保持側の `Child` を渡す形に変え、保持側の死を即検出する。inode は `/proc/locks` の `MAJ:MIN:INO` フィールドの末尾と照合。
4. 足りなかった情報・検証手段: CI 上でのみ再現した競合。ローカル再現は困難（タイミング依存）。
5. 改善案: ロック取得系のテストで「取られているか」を調べるときは、`flock` で触らず `/proc/locks` を読む。これを rust-testing 系スキルに書く価値があるかは次回の教訓化で検討。

## 実装者メモ（round 4）

対象 head: c0601a8d2d34cf44f99ff2b3be49457b3c5d78bb

1. 詰まった箇所: round 3 の Claude レビューが request-changes（must-fix 1 件）: 拒否のテスト 3 件が `--once --dry-run` で動かしているため、初回周回はイベントを出さず dry-run は状態を書かず、「stdout 空」「状態ファイル非作成」が空振りの assert（worktree 分岐の `exit(0)` を消しても通る）。拒否される側を `--once`（dry-run なし）に変更し、拒否されずに動けば状態ファイルが作られる形にした。あわせて偽 `t3` が呼び出しを `fake/t3-calls` に記録するようにし、他インスタンスが無い 2 件で「偽 t3 が呼ばれていない」も確認。変異チェックで worktree 分岐の `exit(0)` を消して `worktree_checkout_is_refused` が落ちることを確認済み。
2. 曖昧だった文: なし。
3. 仮定と解釈: `second_instance_is_refused` は 1 つ目が `--dry-run` で状態を書かないため、2 つ目の `--once` に対して同じ判定が使える。t3-calls は 1 つ目が呼ぶためこのテストでは見ない。
4. 足りなかった情報・検証手段: なし。
5. 改善案: なし（nit は round 3 レビューのまま未対応: `File::try_lock` 化・SAFETY コメント・`file.set_permissions`・`/proc/locks` の dev 照合・dry-run 文書の整合）。

## 実装者メモ（round 5）

対象 head: c9e5b170f48a0f13bafff406f8e831990415c1a5

1. 詰まった箇所: Codex レビュー（環境修復後、head 12b0a4bd に投稿）が request-changes（must-fix 1 件）: 起動元の判定が `opts.project`（監視対象）を見ており、worktree の cwd から `--project <本体 checkout>` で起動するとガードをすり抜ける。`git_dirs` の検査先を `std::env::current_dir()`（起動元）に変更し、`startup_guards` は引数を取らない形に。回帰テスト `worktree_launch_dir_is_refused_for_main_project`（worktree cwd ＋ `--project` 本体 で拒否を確認）を追加し、旧実装への変異で落ちることを確認済み。偽 git は `$PWD/git-dir` マーカー優先・`$FAKE_DIR` フォールバックに拡張。
2. 曖昧だった文: Issue の「起動時に `git rev-parse --git-dir` と `--git-common-dir` を比べる」は実行先ディレクトリを指定していない。`--project` の解釈（起動元か監視対象か）が分かれ得た。起動元（起動コンテキスト）と解釈したのは reviewer 指摘が趣旨（worktree での二重起動防止）に合うため。
3. 仮定と解釈: pitchfork は `pitchfork.toml` のある checkout ルートを cwd にして daemon を起動する前提（`agent-worktree-prune` の cwd 判定と同じ運用）。`--project` は t3/gh 呼び出しの対象を指すまま。
4. 足りなかった情報・検証手段: なし。
5. 改善案: ガードが「どのディレクトリの git 状態」を見るかは Issue の受け入れ条件に明記するとよい（今回はレビューで指摘されるまで起動元と監視対象の区別が表面化しなかった）。

## 実装者メモ（round 6）

対象 head: b792a9941956a47eb9a84289db6ca4899bcc6651

1. 詰まった箇所: 指揮役（代行）の確定仕様に従い round 5 の must-fix を完成。`git rev-parse` を起動元 cwd で 1 回だけ実行し `--git-dir`・`--git-common-dir`・`--show-toplevel` を取得。前二者が違えば worktree として拒否し、さらに正規化した `--project`（相対なら cwd 起点、canonicalize で照合）が toplevel と一致しなければ「`--project` is not the launch checkout」で拒否。拒否ログはパスの種類のみ（lock 拒否行からもパス出力を除去）。テスト (a) worktree 起動拒否・(b) worktree cwd+`--project` 本体・(c) 本体 cwd+`--project` worktree・(d) 本体の正しい起動を網羅。偽 git は `$PWD`/fake 両対応のマーカー方式に `--show-toplevel`（既定 `pwd -P`）を追加。
2. 曖昧だった文: なし（確定仕様の明示どおり実装）。
3. 仮定と解釈: `--project` 不一致は「拒否」（exit 0）とし、canonicalize・rev-parse の失敗は「確認できない」で exit 1（fail-closed）。canonicalize が失敗する `--project`（存在しないパス）は誤設定として 1。
4. 足りなかった情報・検証手段: なし。
5. 改善案: なし。

使ったスキル: implement-issue・review-pr（結果の照合手順として）。輸入スキルはなし。
