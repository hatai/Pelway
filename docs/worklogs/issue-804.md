# Issue #804 の作業ログ

## 実装者メモ（round 1）

対象 head: 28f4c544（実装のコミット列の head。pr-size 例外での 2 段 push のため worklog コミット d70e24a の上に実装を再作成してある。メモのコミットはその後ろに乗るため PR の headRefOid と 1 つずれる）
PR: （draft PR の番号）

1. 詰まった箇所: `use` を丸ごと置き換えると、外した担当の項目が `slots`・`review_limits`・`priority` の並びに残り、重ねた結果が検証で必ず止まる（「使えない担当を外す」という本来の上書きが書けない）。use の置き換え後に、新しい use に無い担当への参照を重ねた結果から落とす（cascade）形にし、落とした項目はキー名つきの警告を出す
2. 曖昧だった文: 「`agents`・`developers` の表は深く重ね、配列は置き換える。上書きできない項目はキー名を伝えて無視するか止める」（Issue #804「受け入れ条件」）
3. 仮定と解釈:
   - 「無視するか止める」は、上書きできないキーはキー名つきの警告（`<file>: warning: <キー>: <理由>`）を標準エラーに出して無視し、重ねた結果が検証を通らないときだけ止める、と解釈した（個々の禁止キーで止めると、雛形に禁止項目を書き残したままの local が全部止まり、運用が進まない）
   - 「必須の役に当たる担当が使えなくなったときに警告」は、マージに要る「現在の head への codex の有効な approve」を満たせなくなる `codex` を `use` から外した場合と解釈し、`developers.<login>.use` に codex が無いとき警告を出す形にした
   - 「自分の `developers.<login>` 以外の表」の判定は、`slots`・`effective` の引数の login を呼び出し側と見なした。`validate` は呼び出し側の login を持たないため、local の `developers.<X>` は共有の toml にある表だけをそれぞれの所有者の上書きとして重ねる（共有に無い login は警告して無視）
   - 「上書きできない」`instance.review` は、main の承認済みの版と `docs/dev-environment.md` の照合に使う共有の値であるため local では変更不可とし、`instance.implement`・`instance.investigate` のみ上書き可とした（開発者の環境の provider インスタンス名に合わせる用途）
   - worktree-setup の `--dest` 既定は `$T3CODE_WORKTREE_PATH`、無ければ cwd、`--from` 既定は `$T3CODE_PROJECT_ROOT`、無ければスクリプトを含む checkout とした（T3 のアクションの実行環境と手動実行の両方に対応）
   - `SLOTS overlay` の行は「あるときだけ出す」を、local ファイルが存在し重ねた結果が通った巡回に限ると解釈した（重ねた結果が検証を通らない巡回は `SLOTS error` が出る）
4. 足りなかった情報・検証手段: T3 の `runOnWorktreeCreate` アクションは環境ごとに Settings → Projects → Actions で取り込む形で、t3.json の変更は取り込み済みの環境に自動反映されない（T3 Discussion #6841）。`.worktreeinclude` は T3 が読まない（Discussion #6905）ため Claude Code・Codex 用の記述とした。いずれも dev-environment.md に手順として書いたが、実際の T3 環境での動作確認はこのスレッドでは行っていない
5. 改善案: `agent-team.toml` の `[agents]` の各担当のコメントに「local では `models`・`effort.raise`・`instance.implement`/`investigate` のみ上書き可」と短く示すと、toml を読んだ人が雛形に辿りやすい（agent-team.toml のヘッダには書いた）

使ったスキル: .agents/skills/implement-issue

## レビュー対応（round 2）

対象 head: 40e1df20
レビュー: claude_review（claude-opus-5-5 / effort=high、run=7195782538）の verdict: request-changes（must-fix 5、nit 6）

must-fix への対応:
1. t3.json — runOnWorktreeCreate が worktree 側（PR の head）の scripts/worktree-setup をホスト権限で実行する経路 → t3.json をスキーマどおり `scripts`/`command` に直し、command を `"${T3CODE_PROJECT_ROOT:?}/scripts/worktree-setup"`（本体の checkout の版を絶対パスで呼ぶ）にした。あわせて発覚したスキーマ違い（`actions`/`script` は t3.json に無い）も解消。dev-environment.md に「本体の checkout を信頼できる版に保つ前提」を追記、試験 8 で command が $T3CODE_PROJECT_ROOT 起点であることを確かめる
2. agent-team.md の `enforce_admins` — 書き換えで `: false` が落ち意味が逆になっていた → main の文言に戻した
3. worktree-setup の --dest 検証 — repo の外への書き出しが可能だった → dst が `--from` と git-common-dir を共有する worktree の最上位であることを実体パスで確かめ、違えば非 0 で止める（verify-run:199・red-check:88-96 の型）。repo 外・worktree 内の途中階層・別 repo を拒否する試験 7b を追加
4. hk のコミット防止の試験 — grep の文言確認だけだった → 一時 repo に hk.pkl を置き `git add -f` して `hk run pre-commit --step no_agent_team_local` が非 0 で止まることを確かめる形に
5. 重ねた結果の検証エラーに local の値が出ていた → `show_values` フラグを足し、local を重ねた結果の検証では値を出さずキー名と理由だけにした（試験 21 で確かめる）

nit への対応（全 6 件対応）: ヘッダコメントの呼び出し元の修正、conductor-rebuild.test.sh の `cmp -s` を check の中へ、試験 18 に effective の use 置き換え確認を追加、cd に `CDPATH=''`、試験 10 に `git ls-files --error-unmatch` による未追跡確認（CI で no_agent_team_local ステップが走らない多層防御）、`agent-team-config` が切れた symlink・通常ファイルでない local を警告して重ねない形に変更


## レビュー対応（round 3）

対象 head: b77ff9ea
レビュー: claude_review（run=7196572338）は approve、codex_review（gpt-6.2-sol / reasoningEffort=high、run=7196538461）は verdict: request-changes（must-fix 11、nit 2）。割れたため request-changes 側を優先して対応

must-fix への対応:
1. worktree-setup のコピーを原子的に — `cat >` による追従・中途半端な上書きを避け、`( set -o noclobber; cat -- "$from" >"$to" )`（O_EXCL、切れた symlink を追わない）に
2. local の symlink 拒否 — `os.path.isfile` は symlink 先を見るため `isfile and not islink` に（conductor-watch.sh も同じ判定に）
3. cascade_use が local が明示したキーまで落としていた → local が書いたキーは共有由来の参照だけ落とす形に追跡を分けた（試験 25）
4. `cascade_use` で `use` の要素に配列・表（hashable でない値）が混ざると `set()` が TypeError になる経路を、str の要素だけ取る形（`{n for n in use if isinstance(n, str)}`。priority の並びにも同じ絞り込み）で塞ぐ
5. warn_required を検証の前に出す順に（use が高リスク担当を全て外したとき、エラーの前に担当不足の警告が出る。試験 26）
6. conductor-rebuild が agent-team.toml 有無で分岐していた → 毎回 worktree-setup を通し local を持ち越す。rebuild 試験の NEW_WT を本物の `git worktree add` に
7. t3.json の command を `bash -c '"${T3CODE_PROJECT_ROOT:?}/scripts/worktree-setup"'` に（既定シェルに依らず POSIX 展開を効かせる）
8. docs の「local から読む」記述を修正 — priority/models は `effective`（重ねた結果）、instance.review・effort.option・承認規則は本体の toml から（implement-issue, command-team, grokbot-runbook ×2, dev-environment）
9. `worktree-setup.test.sh` の引数渡しミス（`$1` 未渡し）と方向の逆だった試験（7c）を修正
10. 文書の文言 pin を board-patrol-docs.test.sh に追加（条件8: 上書き表・effective・needs-decision・worktree-setup）
11. agent-team.md の「値を出さない」を診断出力に限定（nit もあわせて）
push した head: 11e8051f

## レビュー対応（round 4）

対象 head: 140ffc6f
レビュー: claude_review（run=round4）は verdict: request-changes（must-fix 1、nit 2）。codex_review は委託環境の設定の不備（`mcp__t3_code__pull_request_review` 公開）で failed、再送中

must-fix への対応:
1. warn_required を validate_doc の前に移したため型未検証の重ねた結果を読み、`use = 1` で TypeError、`priority = 1` で AttributeError になっていた → warn_required 内で `use` が list でない・`priority`/`priority.review` が dict でないときは警告を飛ばす形に（型の誤りは後続の検証がキー名つきで止める）。試験 27・28 で traceback が出ずキー名を名指すことを確かめる

nit への対応: worktree-setup:126-128 のコメントに「noclobber が防ぐのは通常ファイルの上書き」を明記。worklog round 3 の項目 4・9 の記述を実態に合わせて修正
push した head: e2de46a0（round 4 の対応は b334064a）

## レビュー対応（round 5）

対象 head: 29e084c1
レビュー: codex_review（gpt-6.2-sol / reasoningEffort=high、run=7211295476）は request-changes（must-fix 3、nit 2）、claude_review（claude-opus-5-5 / effort=high、run=7211295991）は request-changes（must-fix 1、nit 3）

must-fix への対応:
1. worktree-setup: dst が本体側の worktree 登録（git worktree list）に載ることを確かめる検査を追加（登録済み worktree の管理 dir を指す .git を持つ未登録ディレクトリを拒否。試験 7d）
2. worktree-setup: 検査したディレクトリを fd で保持し、/proc/self/fd 起点の一時名に noclobber で書き終えてから ln で配置（既存名を上書きせず、失敗時に最終名を残さない）
3. agent-team-config: local を O_NOFOLLOW で開き開いた fd が通常ファイルか確かめてから解析（確認と読み込みの間の置き換え経路を閉じる）。conductor-watch.sh の overlay 表示も同じ判定に
4. docs: レビュー担当・モデル選定の重ねた結果の読み方を「main の承認済みの版（API 取得）を土台に本体の checkout の local を重ね、本体の checkout のスクリプトで effective を走らせる」に統一（implement-issue, command-team ×2, grokbot-runbook ×2。ブランチの toml・local・スクリプトで自己選択できない形に）。effective に <local-file> 引数を追加

nit への対応: 試験 8b を worktree を cwd にして実行、ヘルプの「値を出さない」を警告・検証エラーに限定、conductor-rebuild.test.sh の skip を失敗に（実際の適用は round 6 の対応で）、worklog round 3 の項目 4 を実態どおりに
push した head: dcd6b652（round 5 の対応）

## レビュー対応（round 6）

対象 head: 998365ee
レビュー: claude_review（run=round6、review 6084426321）は request-changes（must-fix 2、nit 2）。codex_review は委託環境の設定の不備で 2 回連続 failed

must-fix への対応:
1. effective の <local-file> 省略時に `<file>` と同じディレクトリ（手順では一時ディレクトリ）の local が読まれるため自己選択に戻る → 手順を「`<本体の checkout>/agent-team.local.toml` を必ず渡す」に変更（implement-issue:257。無ければ共有の値に戻る）
2. local の O_NOFOLLOW open が O_NONBLOCK を欠き、FIFO の local で見張りの本ループが停止する → agent-team-config・conductor-watch.sh の両方に O_NONBLOCK を追加（試験 30 は agent-team-config 側: FIFO local が待たずに警告して重ねない。conductor-watch.sh 側は同じ open 形を試験コードでは確かめていない）

nit への対応: worktree-setup の配置を `ln -T` に（最終名がディレクトリへの差し替えでもその中に作らない）＋コメント修正。conductor-rebuild.test.sh の skip→fail は前回の commit に入っていなかったため今回適用し、round 5 の worklog 記述をその旨に修正
push した head: 3a18d45e（round 6 の対応）

## レビュー対応（round 7 の nit）

claude_review が head 20f521df を approve（review 6084536692）。あわせて挙がった nit 3 件への対応: ヘッダコメントと usage_text に <local-file> 引数を追記、試験 29 に「明示した <local-file> だけが読まれ共有と同じディレクトリの local は重ならない」確認を追加、worklog の試験 30 の範囲の記述を訂正
