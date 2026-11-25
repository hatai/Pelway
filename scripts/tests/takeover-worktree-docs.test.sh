#!/usr/bin/env bash
# 既存の PR ブランチの引き継ぎ手順（Issue #644）の runbook と skill の
# 必須文言を固定するテスト。
# 位置づけ: このスクリプトは「必須文言が文書に存在する」という観測可能な
# 事実だけを確かめる決定的な検査（wording pin）であり、手順の意味や整合性は評価しない
# （AGENTS.md「LLM judges」: 自然言語の意味の評価は rubric 付きの LLM judge が担い、
# 決定的な検査は観測可能な効果に限る。語句が残っていれば矛盾した記述でも通るため、
# 意味の確認は PR 本文での引用と skill review-pr のレビューで行う）。
# 文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
failures=()

# check <name> <file> <pattern...> — すべての pattern が file にあれば合格。
check() {
  local name="$1" file="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF -- "$p" "$ROOT/$file"; then
      failures+=("$name: $file に無い: $p")
    fi
  done
}

CT=.agents/skills/command-team/SKILL.md
II=.agents/skills/implement-issue/SKILL.md
RB=docs/agents/grokbot-runbook.md

# runbook に「実装を始める」の後の節「既存の PR ブランチを引き継ぐ」がある。
check section_exists "$RB" \
  '### 既存の PR ブランチを引き継ぐ'

# 節に t3_thread_launch の workspaceStrategy の形がある。
check workspace_strategy "$RB" \
  '"type": "worktree", "baseRef": "origin/<PRブランチ>", "branch": "<PRブランチ>-r<N>", "startFromOrigin": true'

# 節に同じ内容の CLI の形がある。
check cli_form "$RB" \
  '--worktree origin/<PRブランチ> --branch <PRブランチ>-r<N>'

# 節に <N> の決め方がある。
check n_rule "$RB" \
  'ローカルのブランチと `origin` にある `<PRブランチ>-r<数>` の最大の数に 1 を足したもの（無ければ 1）'

# 節に upstream を向ける最初の作業と、push の形がある。
check upstream_and_push "$RB" \
  'git branch --set-upstream-to=origin/<PRブランチ>' \
  'scripts/agent-gh dev git push origin HEAD:<PRブランチ>'

# 節に、前のスレッドの作業ツリーへの bind は使わず新しい作業ツリーを作ることがある。
check no_existing_worktree "$RB" \
  '`existing_worktree`）は引き継ぎに使わない'

# 節に、修正 round の子は delegate_task（isolation: "worktree"）で立て、
# Claude の Agent ツールの worktree 隔離では立てないことがある。
check fix_round_child "$RB" \
  '`delegate_task`（`isolation: "worktree"`）で立て' \
  'Claude の Agent ツールの worktree 隔離では立てない'

# 節に、新しいスレッド・子への依頼文に PR ブランチ名と fetch・push の形を入れることがある。
check handoff_instruction "$RB" \
  '新しいスレッドへの最初の指示と、修正 round の子への依頼文には、対象の PR の番号（PR がまだ無ければ Issue の番号）、PR ブランチ名と、取り込み・push の形'

# runbook の再開手順の 5 と手当スレッドの項が新しい節を指す。
check resume_pointer "$RB" \
  'open な主 PR があるときはその PR ブランチで引き継ぐ（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'
check assist_pointer "$RB" \
  'その PR のブランチで起動する（立て方は「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」）'

# 再開手順の 5 が、PR が無いときも引き継ぐ成果が残っていれば同じ節の形で
# 引き継ぎ、新規の立て方を引き継ぐ成果が無い場合に限定している。
check resume_no_pr_takeover "$RB" \
  '引き継ぐ成果が残っていれば同じ節の形でその作業ブランチを引き継ぐ' \
  '引き継ぐ成果が無いときだけ、実装スレッドの通常の立て方（「実装を始める」）で起動する'

# 再開手順の 5 が、取得コメントを pelway-dev[bot] 名義のものだけに限り、止まったスレッドの
# branch（別名 -r<N> でありうる）を使わず、特定できないときは確認待ちにする。
check claim_author "$RB" \
  '取得コメントは REST の `user.login` が `pelway-dev[bot]` のものだけを使い、ほかの作成者のコメントは読まない' \
  '`pelway-dev[bot]` 名義の取得コメントが複数あるときは、最新のものを使う' \
  '止まったスレッドの `branch` は使わない' \
  '成果が無いと判断せず起動しない（作業状態の印を確認待ちにして hatai に報告する）'

# 節に、どの経路で得たブランチ名も agent/<Issue 番号>- と英小文字・数字・ハイフンだけかを照合し、
# 外れたら止め、照合を通らない名前をコマンドに入れない決まりがある。
check branch_name_check "$RB" \
  '`agent/<Issue 番号>-`（対象の Issue の番号）で始まり、続きが英小文字・数字・ハイフンだけであることを確かめる' \
  '外れたら起動・委託を止める。照合を通らない名前は' \
  '照合を通らない名前は、成果の有無を確かめる git のコマンド・`t3 thread start`・`git branch --set-upstream-to`・push のどの行にも入れない'

# 引き継ぎ時に items の implement_thread へ PR 番号・thread_id・push 先の組を PR ごとに記録し、
# レビュー結果を送る直前に対象 PR の組で担当を確かめ、確定できなければ保留する決まりがある。
check implement_thread_record "$RB" \
  '手当てする PR の番号（PR がまだ無ければ `none`）・新しいスレッドの `thread_id`・push 先の `<PRブランチ>` の組を書く' \
  '組は PR ごとに持ち' \
  '`implement_thread` で、その PR の番号の組を選び' \
  'そのスレッドの `thread_id` で `implement_thread` の組を書き換える'

# 実装スレッドへ PR についての指示を送るすべての経路で、送る直前に「送り先の照合」を行い、
# 止まった・引き継ぎ済みのスレッドを外し、最初の指示の PR 番号と push 先で割り当てを確かめ、
# 確定できなければ保留する決まりがある（手当スレッドの伝達・人間のレビューの伝達からも指す）。
check send_target_check "$RB" \
  '送り先の照合: 実装スレッドへ PR についての指示を送るすべての経路' \
  '`branch` の一致だけでは送り先を確定しない' \
  '2. 止まっていないこと: 候補が `t3 thread list` にあり、interrupted・cancelled で止まっていないこと' \
  '同じ PR の組が別の `thread_id` に書き換わっているスレッドは引き継ぎ済みとして外す' \
  '3. 割り当てが一致すること: 候補の最初の指示（`t3_thread_read` で読む）が対象 PR の番号を指すか' \
  'かつ、そのスレッドの push 先が対象 PR の `headRefName` と同じであること' \
  '4. 2 と 3 を通った候補がちょうど 1 本のときだけ送り' \
  '0 本か 2 本以上、または最初の指示が読めず割り当てを確かめられないときは、送らずに保留し' \
  '`working`・`stopped` か状態の行が無ければ止まったものとして外す' \
  '`implement_thread` の組の有無を問わず、同じ Issue のスレッドを調べる' \
  'またはその Issue に起動結果が不明なまま照合の済んでいない割り当て ID（「割り当て・停止・再開」の割り当て手順の 4）が残っているときは、どの候補にも送らない' \
  '組はあるが組のスレッドが `t3 thread list` に無いときは、名前の一致で候補を探さずに保留し' \
  '組が無かった・`none` だったときはその PR の番号で組を書く（組の push 先は、照合済みの PR の `headRefName`）' \
  '保留した指示は対応済みにしない' \
  '伝達は「5. イベントごとの対応」の `pelway.agent-watch/v1` の「送り先の照合」を通ったときだけ行い、保留したものは対応済みにしない' \
  'でそのスレッドが対象 PR の現在の担当であることを確かめ、確定できなければ伝達も起動もせずに保留して' \
  '実装スレッドへ伝達する（送る直前に「5. イベントごとの対応」の `pelway.agent-watch/v1` の「送り先の照合」を行い'

# 送り先の照合では、候補を探す前に対象 PR の head が本体リポジトリであることを確かめ、
# 外れたら保留する決まりがある。
check send_target_head_repo "$RB" \
  '候補を探す前に、対象 PR の head を確かめる:' \
  '`isCrossRepository` が `false` と明示されていること' \
  '外れたら、候補を探さず、送らず、組も書かずに保留し'

# 調査の報告 PR は、引き渡しで報告 PR の番号を結び付けた割り当ての記録の行の thread_id を
# 候補にし、GitHub で照合した `pr:` の報告で確かめて送り、停止・引き継ぎ済みを外す決まりがある。
check investigation_send_target "$RB" \
  '調査の報告 PR の送り先: 上の head の確認を通った `agent/investigate-` の PR について' \
  '巡回記録の「割り当ての記録」から、対象 PR の番号が書かれた役割 investigate の行' \
  'その行の `thread_id` を候補とする' \
  '`branch` の一致で候補を探さず、送らずに保留し' \
  '`scripts/agent-gh dev pr view <番号> --json author,headRefName` で、GitHub の対象 PR の作成者が `app/pelway-dev` で、ブランチが `agent/investigate-` で始まり候補の `branch` と同じこと' \
  '前に立ったスレッドは引き継ぎ済みとして外す' \
  '（送り先は上の「調査の報告 PR の送り先」で確かめる）'

# 調査を立てたら割り当ての記録の行に thread_id と作業ブランチを書き、引き渡しで
# 本体リポジトリ・作成者・ブランチを照合した報告 PR の番号をその行に結び付ける決まりがある。
check investigation_launch_record "$RB" \
  '起動したら、割り当ての記録に役割 investigate の行を足し、起動が返した調査スレッドの `thread_id` と作業ブランチ'
check investigation_handoff_bind docs/agents/command-team-details.md \
  'レビューを割り当てる前に、報告 PR を調査スレッドに結び付ける（Issue の有無を問わない）' \
  '報告を返したスレッドの ID が、割り当ての記録の役割 investigate の行に起動時に書いた `thread_id` と同じこと' \
  '--json isCrossRepository,author,isDraft,headRefName' \
  '両方を通ったら、その行に報告 PR の番号を書く'

# implement_thread の組の更新条件に、送り先の照合で組を新規に書く場合がある。
check implement_thread_new_pair "$RB" \
  '送り先の照合の 4 で組が無かった PR の組を書くとき（push 先は照合済みの PR の `headRefName`。同じ照合の 1 の決め方で現在の担当が 1 本に決まったときだけ）、巡回の定型の指示を組の無い PR 前のスレッドへ送るとき' \
  '（「毎回の巡回でスレッドを点検する」。PR の番号は `none`。同じ Issue のスレッドから現在の担当が 1 本に決まったときだけ）に書き換える'

# 別名の作業ツリーで引き継いだときの占有の比較と取得・解放コメントは push 先のブランチで行う。
check claim_branch_push_target "$II" \
  'ローカルのブランチ名ではなく最初の指示に書かれた push 先の `<PRブランチ>` を使う'

# 巡回の定型の指示と、command-team-details の hatai の指摘の伝達も、送る直前に送り先の照合を通す。
check patrol_send_target "$RB" \
  '定型の指示も、送る直前に送り先を確かめる。' \
  'この経路では、照合の 2 の状態の行による除外は当てない' \
  '組が 1 つも無い（この記録の決まりより前に立ったスレッド）ときは、最初の指示（`t3_thread_read` で読む）がその Issue の番号を指し' \
  'そのスレッドを指す `none` の組が無く、別のスレッドを指す組があれば、引き継ぎ済みとして送らない' \
  'ただし巡回の定型の指示と、上の表の `failed` の「続きをやり直す」では、この状態の行による除外だけは当てない' \
  'この 2 つの経路でも、interrupted・cancelled の除外、作業状態の印、引き継ぎ済みの除外、同じ Issue の並行の確認と、3 の割り当ての一致は当てる' \
  '送る直前に、巡回の定型の指示と同じく送り先を確かめる' \
  '「2. ボードと巡回」の「毎回の巡回でスレッドを点検する」の巡回の定型の指示、'
# 組の無い PR 前のスレッドへの巡回の指示は、同じ Issue のスレッドを最初の指示で全部挙げ、
# 挙がったのが 1 本だけでも割り当ての記録の最も新しい行に結び付くスレッドだけを現在の担当とし、
# 1 本に決まるまで（記録上の担当が一覧から欠けている間を含む）送信も組の記録も保留する。
check patrol_no_pair_current_owner "$RB" \
  '組が 1 つも無いときは、さらに同じ Issue のスレッドを全部挙げる' \
  '最初の指示がその Issue の番号を指し、push 先が同じ照合を通るもの（`branch` の名前では挙げない。' \
  '挙がったのがそのスレッド 1 本だけでも、ほかにも挙がっても（' \
  '新スレッドが一覧から一時的に欠けて旧スレッドだけが挙がるときなど）' \
  '割り当ての記録でその Issue の役割 implement の最も新しい行に結び付くスレッド（後から引き継いだスレッド）だけを現在の担当とし、ほかは引き継ぎ済みとして送らない' \
  'その Issue の役割 implement の行が 1 つだけで、挙がったスレッドも 1 本だけのときも、この結び付け方の例外にしない（行に `thread_id`・リンク・割り当て ID のどれも無ければ、そのスレッドには結び付けない。' \
  '結び付くスレッドがちょうど 1 本に決まらない（行が無い、最も新しい行を記録でじかに結び付けられない（' \
  'うちは、挙がったどのスレッドにも送らず、組も書かず、ほかのスレッドを引き継ぎ済み（送り先の照合の 5 の対象外）とも確定せずに保留する' \
  '人間が `needs-human` を外して現在の担当のスレッドを示したら（作成者が hatai の指示に限る）、そのスレッドの `thread_id` と push 先で組（PR があればその PR の番号、無ければ `none`）を書いてから照合し直す' \
  '記録上の担当が一覧から一時的に欠けている間も、再び見えて 1 本に決まるまで同じく保留する' \
  '現在の担当が 1 本に決まってから書く'
# 組の無い PR の送り先の照合も、名前で挙げた候補が 1 本だけでも同じ決め方で現在の担当を決める。
check pr_no_pair_current_owner "$RB" \
  '挙がったのが 1 本だけでも、割り当ての記録でその Issue の役割 implement の最も新しい行に結び付くスレッドだけを候補とし、ほかは引き継ぎ済みとして外す（5 の対象外）' \
  '組の無い PR 前の Issue か組の無い PR で現在の担当が 1 本に決まらない' \
  '組の無い PR 前の Issue か組の無い PR で、「2. ボードと巡回」の「毎回の巡回でスレッドを点検する」の決め方（組の無い PR は 1 の決め方）で別の 1 本が現在の担当に決まった'
# 報告行の表の `pr:` の行は、調査の報告 PR の照合を command-team-details の「調査の引き渡し」に任せ、
# head と 40 桁の sha の決まりを当てない。
check report_pr_row_investigation "$RB" \
  '調査スレッドの報告の PR のときは、この行ではなく `docs/agents/command-team-details.md` の「調査の引き渡し」の照合' \
  'head の一致とこの行の 40 桁の sha の決まりは当てない'
# 保留が続いたときの出口がある。出口に数えるのは現在の担当を確定できない保留だけ。
check send_target_hold_exit "$RB" \
  '保留は、回数に数えるものと数えないものに分ける。数えるのは、現在の担当を確定できない保留だけである' \
  '同じ対象への同じ指示で、数える保留が 2 回続いたら、3 回目の巡回を待たずに 7 のエスカレーションにする' \
  '5 の対象外は保留ではなく、この回数に数えない'
# 現在の担当が別のスレッドに記録で確定しているための外れは、保留と分けて対象外とし、
# 巡回記録に 1 回だけ残し、巡回ごとに確かめ直して当たる間は指示待ちから外し、settle する。
check send_target_excluded "$RB" \
  '5. 対象外: 指示の宛先として見ているスレッド' \
  '確定は 2 と 3 より前に記録だけで決める' \
  '対象外のスレッドには送らず、保留にせず、4 の回数に数えない' \
  '巡回記録には、同じスレッドの対象外の記録がまだ無いときだけ' \
  'その巡回の記録で送り先の照合（「5. イベントごとの対応」の `pelway.agent-watch/v1`）の 5 を確かめ直して対象外に当たったスレッド（' \
  '「送り先の照合」の 5 で対象外（現在の担当が別のスレッドに記録で確定している）に当たるスレッドには、調査の報告 PR でも送らず、保留にもしない' \
  '同じ PR の組が別の `thread_id` に書き換わっているスレッドは引き継ぎ済みとして外す（5 の対象外で、4 の保留の回数に数えない）' \
  '5. settle するその巡回の記録で送り先の照合（「5. イベントごとの対応」の `pelway.agent-watch/v1`）の 5 を確かめ直して対象外に当たったスレッドで、そのスレッドで動いている作業が無いもの'
# 対象外は、照合の済んでいない割り当て ID が残っているあいだは確定せず数えない保留にし、
# 一度の判定で固定せず、指示待ちから外すときと settle するときにその巡回の記録で確かめ直す。
check send_target_excluded_assignment_id "$RB" \
  'その別のスレッドが `t3 thread list` にあり、かつ対象の割り当てに照合の済んでいない割り当て ID が残っていないときである' \
  '対象外にせず、4 の数えない保留（割り当て ID）にする' \
  'どれもこの 5 の条件（割り当て ID の条件を含む）を満たすときだけ対象外とし、満たさないときは 4 の分け方で保留にする' \
  'この記録は経過を残すためのもので、以後の判定には使わない' \
  '対象外は一度の判定で固定しない: 指示待ちから外すとき' \
  '前の巡回で対象外だったスレッドも巡回ごとに確かめ直し' \
  '前の巡回の対象外の記録だけでは settle しない' \
  'その割り当てに照合の済んでいない割り当て ID が残っているときは対象外に当たらないので、settle しない' \
  'ただしその Issue に照合の済んでいない割り当て ID が残っているあいだは、組がまだ現在の担当を表していないので対象外にせず' \
  '書くまでは、照合の済んでいない割り当て ID として送り先の照合の 5 の対象外の確定を止める'
# 組の無い PR 前の Issue と組の無い PR で、割り当ての記録の行とスレッドは記録でじかに
# 結び付け（thread_id・リンク・割り当て ID とスレッド履歴の照合）、日時の近さでは結び付けない。
check no_pair_row_direct_binding "$RB" \
  '行とスレッドは、記録でじかに結び付くときだけ結び付ける: 行に書かれた `thread_id`、行に書かれたスレッドへのリンク、その行の割り当ての起動で残した割り当て ID（「割り当て・停止・再開」の割り当て手順の 4 のクライアント要求 ID）をスレッド履歴と照合して見つかったスレッド、のどれかがそのスレッドを指すこと' \
  'スレッドの立った時刻と行の日時の近さでは結び付けない' \
  '結び付け方（記録でじかに結び付くときだけで、スレッドの立った時刻と行の日時の近さでは結び付けず、行が 1 つで候補が 1 本のときも例外にしない）と、'
# 調査の報告 PR の送り先は、照合の済んでいない割り当て ID が残るあいだ、行を選ぶ前に数えない保留にする。
check investigation_unmatched_assignment_id "$RB" \
  '1. 候補を選ぶ: 行を選ぶ前に、照合の済んでいない割り当て ID（上の送り先の照合の 5 の定義）を確かめる。' \
  '調査 Issue があればその Issue、なければ対象の報告 PR の割り当て（調査の起動と報告 PR の引き継ぎ）にそれが残っているあいだは、行を選ばず、どのスレッドにも送らず、上の送り先の照合の 4 の数えない保留（割り当て ID）にする' \
  '「調査の報告 PR の送り先」の 1 でも行を選ぶ前に保留にして、前の調査スレッドにも送らない'
# 調査スレッドの `finished` は、Draft の報告（起票可否の判断）と Draft 解除後の終わりの報告（マージの判断）で分ける。
check investigation_finished_split "$RB" \
  'ただし `finished` の `pr:` は、6 の `pr:` の行で確かめた PR の状態で扱いを分ける。' \
  'Draft の報告 PR を指す skill `investigate-bug` の手順 5 の報告（`ready: done` が無い）なら、レビューを始める前に' \
  '起票可否の判断は Draft の解除の前に済んでいるので、上の表の `finished` のとおりマージの判断に進む'
# 調査の報告 PR を引き継いだときは、割り当ての記録に新しいスレッドの行を書き、送り先の照合はその行で確かめる。
check investigation_takeover_record "$RB" \
  '調査の報告 PR（`headRefName` が `agent/investigate-` で始まる）を引き継いだときは、`implement_thread` の組の代わりに、割り当ての記録に役割 investigate の行を足し' \
  '引き継ぎで立てた行の候補は、`branch` が `<PRブランチ>-r<N>` で返答に `pr:` が無いことがあるため'
# 調査の報告 PR を引き継ぐスレッドの最初の指示で従わせる skill と、その返答の形での判定がある。
check investigation_takeover_skill "$RB" \
  '報告 PR のレビューを待つ間（修正を push した後など）は `state: waiting-conductor`・`head:`・`next:` の 3 行の形で書かせ' \
  'Draft を解除し終えたら同じ skill の手順 10 の終わりの報告の 6 行（`issue:`・`pr:`・`review:`・`ready: done`・`memo:`・`state: done`）で書かせる' \
  '`review:` は指揮役が立てた報告 PR のレビューのラウンド数と `approve`、`memo:` は `none`（報告 PR は報告のファイルだけで、実装者メモを書かないため）' \
  '巡回の定型の指示（「続きをやり直す」など）を受けたときは、GitHub の実状態で作業を決めさせる' \
  '終了通知（5 の表の `finished`）は、実装スレッドと同じく `pr:` と `ready: done` を読んでマージの判断に進む' \
  '同じ照合のうち Draft の代わりに Draft でないこと（`ready:` の行）で確かめ' \
  '調査の報告 PR を引き継いだスレッドの終わりの報告の `memo: none` は確かめず' \
  '最後の返答の状態の行（同じ節で最初の指示に従わせる skill `implement-issue` の「ターンの終わりの状態の行」）'
check investigation_takeover_record_details docs/agents/command-team-details.md \
  '報告 PR を引き継いで立てたスレッドの行は、起動時に報告 PR の番号を書く'
check details_send_target docs/agents/command-team-details.md \
  '送る直前に `docs/agents/grokbot-runbook.md` の「5. イベントごとの対応」の `pelway.agent-watch/v1` の「送り先の照合」を行い'

# 別名のブランチから初めて Draft PR を作るときは --head で push 先を指定し、作成後の
# headRefName を push 先と照合する決まりが implement-issue の手順 7 にあり、runbook の
# 最初の指示と再開手順の 5 がそれを指す。
check draft_pr_head "$II" \
  '`pr create` に `--head <PRブランチ>` を付け、push 先のブランチを PR の head に指定する' \
  'が、最初の指示に書かれた push 先の `<PRブランチ>` と同じことを確かめ、違えば止まる条件と同じ扱いにする'
check draft_pr_head_runbook "$RB" \
  'Draft PR を `scripts/agent-gh dev pr create --draft --head <PRブランチ>` で作り、作成後の `headRefName` が `<PRブランチ>` と同じことを確かめる' \
  '`--head <PRブランチ>` で作り、作成後の `headRefName` を `<PRブランチ>` と照合することを入れる' \
  '対象の PR の番号（PR がまだ無ければ Issue の番号）、PR ブランチ名と'

# 節に、起動・委託の前に PR の head リポジトリと ref を照合し、外れれば止める決まりがある。
check head_repo_guard "$RB" \
  '--json isCrossRepository,headRefName' \
  '`isCrossRepository` が `false` と明示され（`true`・値が無い・読めないときは外れ）、`headRefName` が `<PRブランチ>` と同じときだけ進む。外れたら起動・委託を止める'

# implement-issue の手順 6 に、別名ブランチのときは両方の push を HEAD:<PRブランチ> に
# 置き換える決まりがある。
check implement_push_target "$II" \
  '`scripts/agent-gh dev git push origin HEAD:<PRブランチ>` に置き換える'

# implement-issue の手順 9 に、修正 round を子に任せるときは runbook の節に従う 1 文がある。
check implement_issue_pointer "$II" \
  '修正 round を子に任せるときは、`docs/agents/grokbot-runbook.md` の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節に従う'

# command-team の「手当て・引き継ぎの前の確認」に、runbook の節の形で立てる 1 文がある。
check command_team_pointer "$CT" \
  'PR がすでにあり、そのブランチが `origin` にある引き継ぎ・再開・手当てのスレッドは、runbook の「4. 仕事の始め方」の「既存の PR ブランチを引き継ぐ」の節の形で立てる'

if ((${#failures[@]})); then
  printf 'FAIL: %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "takeover-worktree-docs: ok"
