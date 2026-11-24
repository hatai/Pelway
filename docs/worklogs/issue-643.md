# Issue #643 の作業ログ

## 実装者メモ（round 1）

対象 head: 086f9889f3cdf2a9bb953fbf47db4f0a92447015

1. 詰まった箇所: `scripts/tests/conductor-watch-stall.test.sh`: 既存のモックの `t3` が `thread list` 以外を `other-calls` に記録するため、新たに呼ぶ `thread show` で `no_writes` が落ちた。読み取りなのでモックに `thread show` の応答（空の作業ツリー）を足した。sandbox 内では実物の `t3 thread list --json` が空の出力で JSON にならず、`thread show` の形は Issue の記述（`thread.worktreePath`）に拠った。
2. 曖昧だった文: 「`t3 thread show` が失敗したスレッドはその巡回で数えず、ほかのスレッドの判定と見張りの周回は続く」（Issue #643 の受け入れ条件）
3. 仮定と解釈: 失敗したスレッドを含む既出の組み合わせは通知済みの記録から消さないことにした（消すと、失敗が直った次の巡回で同じ行が再び出る）。失敗中に残りの本数で別の組み合わせができたときは、その組み合わせを 1 行出す。目的の「トップレベル」に合わせ、`parentThreadId` があるスレッドも数えない（一覧には現れない前提）。
4. 足りなかった情報・検証手段: sandbox から実行中の T3 サーバーの `t3` に届かず、実物の `thread show --json` の出力を確かめられなかった。
5. 改善案: `docs/agents/grokbot-runbook.md` の「`pelway.agent-watch/v1`」の表の直後に「`THREAD shared-worktree` の組み合わせは、`thread show` の失敗中も通知済みとして保つ」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: ab51a167208edf8e27099d1cb83a1077120850a1

1. 詰まった箇所: `scripts/conductor-watch.sh`: round 1 は `if threads:` で判定を飛ばしており、一覧の取得失敗と正常な空の一覧（`[]`）を区別していなかった（codex の must-fix）。`thread list` の失敗時に `threads.json` を消し、python 側で `threads_ok` を立てて区別した。試験 6 を足した。
2. 曖昧だった文: 「同じ PR の 2 回目以降のレビューは 1 段上げる」（`.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」）と「3 回目のレビューの委託では effort を 1 段上げる」（`.agents/skills/implement-issue/SKILL.md` の手順 9）
3. 仮定と解釈: effort は、実装担当が担当と effort を決めるときに従う表として command-team の文を採り、round 2 の委託から 1 段上げた（codex low→medium、claude medium→high）。claude の nit 4 件は `[文書のずれ]` の印が無いため直していない。
4. 足りなかった情報・検証手段: round 1 と同じく、実物の `t3 thread list`/`thread show` の JSON（`createdAt` の有無を含む）を sandbox から確かめられない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「3 回目のレビューの委託では effort を 1 段上げる」を、command-team の「2 回目以降は 1 段上げる」と揃える。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 70bee051b912f2bb089f52ef5ef4b607b9ffb9cc

1. 詰まった箇所: `scripts/conductor-watch.sh`: round 2 は `thread show` が失敗したスレッドを含む組み合わせを無条件に残していた（codex の must-fix）。残りのスレッドが同じパスにいると確かめられたときだけ残し、記録の型を `stall_notified` と同じ JSON オブジェクト（パス → スレッドの組）にした。試験 7 と型の確認を足した。runbook の行は claude の `[文書のずれ]` の nit で、3 本以上でも読める書き方にした。
2. 曖昧だった文: 「見張りの状態ファイル `state.json` に記録する。`stall_notified` と同じ型」（Issue #643 の受け入れ条件）
3. 仮定と解釈: 「同じ型」を「JSON オブジェクト（文字列のキー → 文字列の値）」と読み、キーをパスにした（同じパスに同時に 2 組はできないため）。round 1 では「通知済みの記録で重ねて出さない」という役割の同じさと読んで配列にしていた。同じ種類の must-fix（失敗と解消の区別）が 2 round 続いたので、記録を保つのは「失敗したスレッドを含み、かつ残りが同じパスにいると確かめられた」ときだけ、という決まりを一覧の取得失敗と `show` の失敗の両方に当てた（一覧の取得失敗は判定を飛ばして全体を保ち、それ以外は確かめられた事実で消す）。runbook の `createdAt` の欄名は、実物で確かめられないので外した。
4. 足りなかった情報・検証手段: 実物の `t3 thread show --json` に作成時刻の欄があるかを sandbox から確かめられない。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「状態の型を既存の値に合わせるときは、型（JSON のオブジェクト・配列など）とキーを書く」と足す。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: 005194114b5290092aba5b20b1171c3c426c70c2

1. 詰まった箇所: `scripts/conductor-watch.sh`: round 3 までは `shared_notified` がパス → 直近の組み合わせの 1 件だけだったため、失敗するスレッドが巡回ごとに入れ替わると別の組み合わせで上書きされ、共有が続く同じ組み合わせを再通知していた（codex round 3 の must-fix）。記録をパス → 組み合わせごとの入れ子の JSON オブジェクトに変え、入れ替わりの試験 8 を足した。`mise run verify` では `.agents/skills/verify-pelway/verify-run.test.sh` の `make_run_dir`/`locks_dir_ready` 系 6 件だけが失敗した。sandbox の `/tmp` が uid 65534 所有のため `dir_chain_safe` が拒む環境起因で、変更を退避した同じツリーでも再現したため、その内訳と再現確認を PR 本文に書いて進める。
2. 曖昧だった文: 「パスとスレッド集合の組ごとに通知済みの記録を保持できる JSON オブジェクトにし」（PR #656 の codex round 3 レビューの must-fix）
3. 仮定と解釈: 「組ごとの記録」を「パス → {組み合わせ文字列: 1}」の入れ子と読み、round 2 で置いた「パス → スレッドの組」（平坦な文字列 → 文字列）の型の読みをこの指摘で更新した。旧形式の記録は読み替えてアップグレード直後の再通知を防ぐ。開発リードの指示（ラウンド上限の撤廃・claude_review を代用せず未完了とする）は「hatai の指示がこの手順と食い違うとき」に当たると解釈し、指示の範囲（must-fix の修正と修正⇔レビューの継続）だけを行う。`pr ready`・マージ・force push は指示にも無く行わない。
4. 足りなかった情報・検証手段: 撤廃の一次情報（`t629u` の実体）。main の `.agents/skills/implement-issue/SKILL.md` にはまだ上限の記述があり、docs では判断を裏付けられなかった。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の表と手順 9 の「3 回目のレビュー結果に must-fix が残っていたら…エスカレーションする」に、ラウンド上限の撤廃が正式に決まったときはその記録へのリンクを足す。

使ったスキル: implement-issue

## 実装者メモ（round 4 追記: semgrep 検出）

対象 head: 72299bf5c09050ce82a96f86ec62495163fa1854

1. 詰まった箇所: `semgrep (code patterns)` が `532c21ea` で失敗した。main の `8da28305`（INFO 以外の検出をジョブ失敗にする）が merge ref に入ってから初めての走査で、前の head は失敗にしない旧スクリプトで走っていたため見えていなかった。検出は `bash.lang.security.ifs-tampering`（WARNING）の 1 件で、場所は `scripts/tests/conductor-watch-shared-worktree.test.sh:56` の `local IFS=,`（round 1 の新規ファイルに初めからあった行）。`threads()` を `printf '%s,'` で連結する形に変えて IFS を触らないようにした。検出の一覧はジョブサマリーにしか出ず API では取れないため、`refs/pull/656/merge` をフェッチして別 worktree に置き、mise の `pypi:semgrep@1.186.0`（CI と同じ版）で同じルールセットを実行して再現した（検出 35 → 34 件、WARNING 以上は 0 件になった）。
2. 曖昧だった文: 「手元で同じコマンドを実行して確かめる」（`security-scan.sh` のヒント）
3. 仮定と解釈: 「同じコマンド」は `security-scan.sh` 内の `semgrep scan` 行と同じ引数で、対象ツリーは CI と同じ merge ref と読んだ。sandbox のホームは読み取り専用で `~/.semgrep` に書けないため `XDG_CONFIG_HOME=/tmp/xdg` で設定先を変え、イメージではなく mise に入っている同じ版を使った。
4. 足りなかった情報・検証手段: ジョブサマリーは API の check-runs/annotations やログには出ないため、検出の規則・場所・重大度は手元の再現でしか取れなかった。
5. 改善案: `docs/dev-environment.md` の「セキュリティスキャン（security-scan）」に「検出の一覧はジョブサマリーに出る。手元の再現は merge ref の worktree に対して `XDG_CONFIG_HOME` を書ける場所に向けて `mise x pypi:semgrep@<版>` で同じ引数の `semgrep scan` を実行する」と足す。

使ったスキル: implement-issue
