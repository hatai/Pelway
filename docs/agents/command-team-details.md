# Command Team: 条件つきの詳細手順

skill `command-team` から、特定の条件でだけ必要になる詳細を移した文書。各節の冒頭に読む条件を書く。用語「巡回記録」は skill `command-team` の定義（指揮役の作業ログ `docs/worklogs/` の「割り当ての記録」と「判断の記録」）に従う。

## 週次: 長命 PR の棚卸し

読む条件: 前回の棚卸しから 7 日以上経った巡回（週次巡回）。

開いている PR の棚卸しをする。前回の実施日は巡回記録で確かめる。

- 判定には GitHub の `updated_at`（`scripts/agent-gh dev pr list --json number,updatedAt,state`）と巡回記録を並べて使う。GitHub 側だけ見ると、指揮役が待たせる理由を決めている PR を「進展が無い」と誤認し、巡回記録側だけ見るとコミットやレビューの動きを書き漏らすためである。
- 「進展」は次のいずれか。進展が一定期間（目安 7 日）無い PR を棚卸しの対象にする。
  - コミットの追加
  - レビューの投稿（Approve・Request changes）
  - レビューコメント
  - CI 状態の変化（`updated_at` に現れないことがあるので `scripts/agent-gh dev pr list` の `statusCheckRollup` で確かめる）
  - 巡回記録への状態注記（待たせる理由や期限を指揮役が書いたとき。注記した日を進展と数える）
- 対象にした PR ごとに、判断期限（次の週次巡回まで）付きで「残す」「作り直す」「閉じる」のどれかを決め、判断と期限を巡回記録の「判断の記録」に書く。
  - 残す: 理由（例: 依存する PR のマージ待ち）と次の判断期限を注記する
  - 作り直す: 実装担当に新しいブランチ・PR で作り直させる
  - 閉じる: PR を閉じて Issue を `ready-for-human` か `wontfix` で整理する（`docs/agents/triage-labels.md`）
  - PR のクローズは指揮役の「しないこと」（runbook 1）なので、「作り直す」「閉じる」の操作は人間に依頼する（PR に依頼のコメントを書き、部屋で人間に知らせる）
- 判断期限を過ぎても判断が残っている PR は、次の巡回で runbook 7 のエスカレーションにする。
- 7 日という期間は目安であり、運用実績で見直す。

## 外部からのレビュースレッド投入

読む条件: レビューの割り当てが滞留したとき（「レビュー待ちの滞留」の対応として外部投入が必要なとき）。

正本の指揮役の代わりに、別の指揮役・代行者・人間が許可した外部のエージェント（例: Muse）がレビュースレッドを投入してよい。場当たりに行わず、次の手順で行う。

1. 投入前に、その PR に既存のレビュー担当が居ないか確かめる。`t3 thread list` でその PR のブランチ（`review/<PR 番号>-`）のレビュースレッドを探し、`scripts/agent-gh dev pr view <PR 番号> --json reviewDecision,reviews` で投稿済み・進行中のレビューを確かめる。担当が居れば投入しない。担当が居るのに止まっているときは、正本の指揮役（居なければ人間）に知らせる。
2. 滞留の証拠（`pr-digest` の `review-wait` 行）を確かめてから、レビュースレッドを立てる。立て方は runbook の「レビューを始める」と同じ（skill `review-pr`、権限は `auto-accept-edits`、worktree は main から、スレッド名は「スレッド名」の形式）。
3. 投入した事実を巡回記録に残す。投入した者が PR に投入の旨（誰が・いつ・なぜ）をコメントし、指揮役は次の巡回でそれを指揮役の作業ログの「割り当ての記録」の表に 1 行足す（日時、PR、役割 review、投入した者、モデル、effort、理由は「レビュー滞留への外部投入」）。投入されたスレッドは `t3 thread list` で見えるので、レビューの同時数の上限（runbook 3）に数える。

## 調査（入口B）

読む条件: 入口B（原因が未理解のバグ・インシデント報告）で調査スレッドを立てるときと、調査スレッドが報告を出したとき。どの報告が入口B かは、正本 `docs/superpowers/specs/agent-team.md` の「タスクの入口と流れ」。

### 調査の担当と effort

- 担当は GLM（opencode `zai-coding-plan/glm-5.3`）。実装スレッドと合算して同時 3 本の枠で数える（正本の「制約」）。数え方は、`agent:working` が付いた Issue の数と実行中の調査スレッドの数の合計
- 枠が満杯のときは指揮役が優先度を判断する。原則として、本番インシデントの調査をほかの実装や調査より先に置く
- effort は、調査は読み取り中心のため既定。対象が広い（複数のクレートやサブシステムにまたがる等）場合は 1 段上げる
- 割り当ての記録の表には、役割 investigate で書く

調査スレッドを立てる指示は次の形にする（2 行目に割り当ての行を必ず入れる）:

```
<報告の対象> を調査する。skill investigate-bug に従う。
Assigned: role=investigate agent=opencode model=zai-coding-plan/glm-5.3 effort=<low|default|high|max>
```

### 調査の引き渡し

調査スレッドが報告を出したときの処理。

- 報告は `docs/worklogs/bugs/<slug>/report.md` と、そのファイルだけの Draft PR（skill `investigate-bug` の手順 5）。PR とスレッドの返答の両方で報告を確認する
- 調査 Issue（ボード上の調査タスク）では、報告 PR がそのまま主 PR になる。報告 PR の本文には `Part of #<番号>` を必ず書かせること（`Part of` はクローズ用キーワードではないため、報告 PR のマージを検知したら指揮役が Status を Done にする）
- 指揮役が報告を確認して起票可否を判断し、判断と理由を報告の PR にコメントする
- 直す必要があるときは、指揮役が skill `write-issue` で起票する（実装の指示は出さない）。人間は `ready-for-agent` で承認し、入口A の流れに合流する。Issue 本文の「関連情報」には、報告のパス（`docs/worklogs/bugs/<slug>/report.md`）と調査スレッドのリンクを書く
- 直す必要がないとき（仕様どおり、誤報告など）は Issue を立てない
- 起票の可否にかかわらず、報告の PR はレビューに回してマージする（報告が main に残る）。レビューの割り当ては通常の PR と同じ「レビュー担当と effort」で行い、Draft の解除は調査スレッドにさせる（実装スレッドと同じ流れ）
- 調査で報告に不足があって `needs-info` を付けたとき（Issue の無い報告では、スレッドの返答に質問が書かれ、報告の PR はまだ無い。質問を報告者に取り次ぐ）は、情報が届いたら原則として同じ調査スレッドを再開する（`t3_thread_send` で「続きを調査する。skill investigate-bug に従う。」）。新しい調査スレッドは立てない
- 実装は、調査を担ったスレッドへの役割の追加としては行わない。起票した Issue の実装スレッドとして割り当て替える（正本の「役割と GitHub 上の名義」の役割の変更の原則）
- 調査スレッドは脆弱性の疑いで止まることがある（skill `investigate-bug` の手順 4）。このとき報告の PR は無いので、`needs-human`（Issue の無い報告ではスレッドの返答）への人間の判断を待つ

## hatai のレビューの扱い

読む条件: hatai へのレビュー依頼（「hatai 待ち」）をするとき、または hatai が紐づく PR にレビュー・コメントを投稿したとき。巡回での検知と Status の反映は runbook の「ボードと巡回」の「hatai のレビューに関する連動」が正本で、ここには指揮役が判断に使う基準だけを置く。

「hatai 待ち」:

- 依頼は主 PR に対してだけ行い、主 PR に定型コメント「[review-request-human] PR #<主 PR の番号>」を書いて記録する。取り消しは「[review-request-human] cancel PR #<番号>」。hatai が依頼した場合は、元の発言へのリンクを含めて指揮役が主 PR への依頼として記録し直す。判定は最新の定型コメントだけを見る
- 完了証跡がそろい `needs-human` が付いていない「hatai 待ち」の Issue は、Status を Ready to merge に置く（完了証跡の対象別の内訳は runbook）
- 依頼が終わるのは、その PR に hatai がレビューを投稿する・マージされる・Issue が閉じる・取り消しが書かれる、のいずれかで、最新の依頼コメントより後に起きたイベントだけが効く

hatai の指摘の伝達:

- hatai が紐づく PR（open のもの。主 PR 以外も含む）にレビュー・コメントを投稿したら、紐づく Issue の Status を Changes requested にし、実装スレッドに `t3 thread send` で伝える（`needs-human` 付きの Issue には手当スレッドを起動せず、伝達と対応済みの記録だけを行い、Status は Needs human のまま）
- 実装スレッドが対応（修正の push、または対応不要の返答）したら、指揮役が確認して指摘の ID を補助記録の対応済みに残す。対応済みになった時点で Status を導出値に戻す

## Dependabot の PR の扱い

読む条件: Dependabot が立てた依存更新の PR を扱うとき（「マージ」の一部）。

- Dependabot が立てた依存更新の PR（`dependencies` ラベル、`build(deps):` で始まるコミット）のうち、minor と patch は、通常の PR と同じレビュー（skill `review-pr`）とマージの決まりで扱う。Dependabot の PR に `automerge` ラベルを付けるのも指揮役だけであり、Kodiak の `merge.automerge_dependencies`（レビューなしの自動マージ）は使わない。
- Dependabot の major と 0.x の更新は直接処理しない。移行 Issue（更新の影響範囲と検証内容を書く。skill `write-issue`）を切って通常フロー（入口A）へ回し、Dependabot の PR 自体はクローズせず残す（クローズは人間の操作）。
- Action の更新の PR（`ci(deps):` で始まるコミット）では、Dependabot は `.github/workflows/actions.lock` を作り直さないため、指揮役が `automerge` ラベルを付ける前にその PR のブランチを checkout して `gh actions-lock` を実行し、生成し直した `actions.lock` を同じブランチに push してから CI の通過を確かめる（`gh actions-lock` は人間の名義の `gh` 拡張で、App のトークンでは動かせない）。この操作は、Workflows 権限が必要なために人間の名義で行う例外の 1 つで、例外の正本は `docs/superpowers/specs/agent-team.md` の「制約」。

## レビュー担当の段階移行

読む条件: 抜き取りの集計の時点が来たとき、または複数レビュー体制の有効化を判断するとき。

抜き取りの実施が 10 件に達するか、抜き取りの開始から 4 週間が経つか、どちらか早い方になったら、初回の見落とし率を集計する（定義は skill `command-team` 手順 4 の「Codex・Devin CLI の見落とし」）。その集計をもって高リスク PR の複数レビュー（Codex と Claude、確認済みなら Devin CLI も）を有効化し、有効化したことと集計結果を指揮役の作業ログに記録する。複数体制は段階的に有効化し、まず抜き取りから始める。抜き取りの実施件数は PR のレビュー（Claude が付けられた PR の数）で数え、集計のときに作業ログに残す。

## 代行の開始と終了

読む条件: 正本の指揮役（Grok Bot）が動かず、人間が代行を頼んだとき（開始）と、正本へ戻すとき（終了）。skill `command-team` の手順 5 から参照される。

### 開始のチェック

1. Grok Bot が動かないことを人間が確かめ、代行を頼んだ
2. T3 のスレッドとして動くなら `t3_thread_launch` で作業スレッドを、それ以外なら T3 Connect の MCP につながる（レビューの権限 `auto-accept-edits` を指定してスレッドを立てるために必要）
3. `scripts/conductor-watch-ensure.sh` を実行した
4. 引き継ぐ状態を読んだ: `agent:working` の Issue、Draft・レビュー待ちの PR、実行中のスレッド、`needs-human` の Issue、指揮役の作業ログ
5. 代行の作業ログ `docs/worklogs/<日付>-conductor-substitute.md` を作った（判断と改善点を書く）
6. T3 のスレッドとして動くなら、hatai がホストの端末で `scripts/conductor-register <自分のスレッド ID>` を実行した（agent-watch が受信箱に新着を書いたときに起こす宛先。登録ファイルは sandbox から書けない `~/.config/pelway-agent-notify/conductor-thread-id`。`docs/dev-environment.md` の「pitchfork について」）

### 終了のチェック（正本の指揮役へ戻す）

1. 新しいスレッドを立てるのを止め、hatai がホストの端末で `scripts/conductor-register --clear` を実行する
2. 作業ログに、進行中の Issue・PR、待っているもの、未反映の改善点を書く
3. 作業ログを PR にしてマージする
4. 人間に、作業ログの場所と、戻した時点の状態を報告する
5. 見張りを止めた（systemd unit で動いていれば `systemctl --user stop pelway-conductor-watch.service`、直接起動ならプロセスの kill）
