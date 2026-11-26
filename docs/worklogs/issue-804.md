# Issue #804 の作業ログ

## 実装者メモ（round 1）

対象 head: 29cc24be（実装のコミット。pr-size 例外での 2 段 push のため、基底の worklog コミット d70e24a の上に再作成したもので、メモのコミットはその後ろに乗るため PR の headRefOid と 1 つずれる）
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
