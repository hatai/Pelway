# 人間が直接行う操作の一覧

AI 開発チームの運用で、人間（hatai）が直接行う操作を正本の文書から拾い、操作ごとに Grok Bot 経由にできるかの提案を付けた一覧。経路を実際に変えるのは、hatai が末尾の「hatai が決めること」で選んだものごとに、別の Issue で行う。

## 数えた基準

- 基準のコミット: `3afed98359f4d521166cd1faa9cb63b9f64a2ad2`。この文書の `パス:行番号` は、すべてこのコミットの行番号
- 対象のファイル（12 個）: `AGENTS.md`、`docs/superpowers/specs/agent-team.md`、`docs/agents/grokbot-runbook.md`、`docs/agents/command-team-details.md`、`docs/agents/conductor-context.md`、`docs/agents/triage-labels.md`、`docs/dev-environment.md`、`.agents/skills/command-team/SKILL.md`、`.agents/skills/write-issue/SKILL.md`、`.agents/skills/implement-issue/SKILL.md`、`.agents/skills/investigate-bug/SKILL.md`、`.agents/skills/review-pr/SKILL.md`
- 拾う行: 基準のコミットで次の `git grep` に当たる行の全部

```bash
git grep -n -E 'hatai|人間|濱田|human' 3afed98359f4d521166cd1faa9cb63b9f64a2ad2 -- \
  AGENTS.md \
  docs/superpowers/specs/agent-team.md \
  docs/agents/grokbot-runbook.md \
  docs/agents/command-team-details.md \
  docs/agents/conductor-context.md \
  docs/agents/triage-labels.md \
  docs/dev-environment.md \
  .agents/skills/command-team/SKILL.md \
  .agents/skills/write-issue/SKILL.md \
  .agents/skills/implement-issue/SKILL.md \
  .agents/skills/investigate-bug/SKILL.md \
  .agents/skills/review-pr/SKILL.md
```

- 当たった行: 243 行（`docs/dev-environment.md` を除く 11 ファイルで 206 行、`docs/dev-environment.md` で 37 行）
- 当たった行ごとに、人間が直接行う操作かを次の決まりで判定し、操作なら一覧の行の「出どころ」に、操作でなければ「操作ではない行」に入れる。1 行が複数の操作に当たるときは、当たる一覧の行すべての出どころに書く
  - 操作にするもの: 人間が手で行う操作、人間の名義か資格情報で行う操作（指揮役が人間の名義の `gh` で行うものを含む）、人間の判断が出るまで作業が止まる判断と、その判断の記録
  - 操作ではないもの: hatai がパス・URL・リポジトリ名の一部としてだけ現れる行、ラベル名や Status 名（`needs-human`・Needs human など）だけの言及、人間に知らせる・報告するだけの行、人間の側に操作が生じないエージェントの手順
- 出どころと「操作ではない行」の `パス:行番号` は、どれもバッククォートで囲んだ 1 語で書く。漏れは次で数え直せる（出力が無ければ漏れは 0 件）

```bash
git grep -n -E 'hatai|人間|濱田|human' 3afed98359f4d521166cd1faa9cb63b9f64a2ad2 -- <対象のファイル> | cut -d: -f2,3 | sort -u > hits.txt
grep -oE '`[^`]+:[0-9]+`' docs/agents/human-actions.md | tr -d '`' | sort -u > cited.txt
comm -23 hits.txt cited.txt
```

- 提案の値
  - できる: 人間が部屋で Grok Bot に伝え、Grok Bot が操作すれば足りる
  - 条件付き: 書いた条件を満たすときだけ Grok Bot 経由にできる
  - できない: 理由が権限（GitHub App の権限、admin）、PC の物理的な操作、`#603` の必ず人間に聞く範囲のどれかに当たる。理由の欄の先頭にどれに当たるかを書く
- `#603` は、実装担当の止まりと承認の待ちを Grok Bot に渡す流れの Issue。`#603` の必ず人間に聞く範囲は、`.github/workflows/` の変更と push、sandbox の外での実行、秘密・外部への接続、admin マージ、名義の異常、Prior art first の方式の選択
- 「部屋」は人間と指揮役の会話（runbook の「11. 代行」では、代行中の代行者と人間の会話を指す）

## 一覧

| ID | 人間がすること | 出どころ | 今の経路 | 提案 | 理由 | 経路を変えるのに要る変更 |
|---|---|---|---|---|---|---|
| H01 | Issue を起票する（Issue Form `agent-task`、skill `write-issue`） | `docs/superpowers/specs/agent-team.md:18`、`docs/superpowers/specs/agent-team.md:30`、`.agents/skills/write-issue/SKILL.md:13` | GitHub: Issue Form で Issue を書く | できる | 指揮役はすでに skill `write-issue` で起票している（skill `command-team` の「決まり（毎回守る）」）。人間が部屋で頼んだ内容を Grok Bot が起票すれば足り、入場の承認は H02 に残る | 文書（仕様の「人間が行うのは次の 3 つだけ」の 1） |
| H02 | 起票した Issue に `ready-for-agent` を付ける（入場の承認） | `docs/superpowers/specs/agent-team.md:18`、`docs/superpowers/specs/agent-team.md:30`、`docs/superpowers/specs/agent-team.md:103`、`.agents/skills/write-issue/SKILL.md:24`、`.agents/skills/write-issue/SKILL.md:187`、`.agents/skills/command-team/SKILL.md:48` | GitHub: Issue のラベルを付ける。指揮役が付けるのは人間に委任されたときだけ | 条件付き。条件: 人間が部屋で承認を述べ、Grok Bot がその言葉の引用・述べた時刻・目印（`#603` の `<!-- grokbot-human-decision -->` と同じ形）を含むコメントを Issue に書いたとき | 委任の枠（skill `write-issue` の「指揮役は人間に委任されたときだけ」）がすでにあり、委任の記録の形を決めれば足りる | 文書（skill `write-issue` の手順 6・7、仕様の「状態の表し方」、`triage-labels.md`） |
| H03 | 調査の報告の後、指揮役が起票した Issue を `ready-for-agent` で承認する | `AGENTS.md:33`、`docs/superpowers/specs/agent-team.md:54`、`docs/agents/command-team-details.md:61` | GitHub: 報告の Draft PR を読み、起票された Issue にラベルを付ける | 条件付き。条件: H02 と同じ記録のコメントに、報告の PR へのリンクを含めたとき | 起票の可否は指揮役が報告を読んで判断している（`command-team-details.md` の「調査（入口B）」）。人間に残るのは承認だけ | 文書（AGENTS.md の「Agent workflow」、仕様の「入口B（バグ・インシデント）の調査と起票」、`command-team-details.md` の「調査（入口B）」） |
| H04 | ボードのカードを動かす（Backlog から Todo へ動かして処理の合図、Backlog へ戻して停止、Done へ動かして閉じる）、Issue を直接閉じる、着手後に `ready-for-agent` を外す | `docs/superpowers/specs/agent-team.md:18`、`docs/superpowers/specs/agent-team.md:30`、`docs/superpowers/specs/agent-team.md:41`、`.agents/skills/write-issue/SKILL.md:25`、`docs/agents/grokbot-runbook.md:51`、`docs/agents/grokbot-runbook.md:53`、`docs/agents/grokbot-runbook.md:77` | GitHub: Projects のボード（`users/hatai/projects/7`）のカードを動かす、Issue を閉じる | 条件付き。条件: 人間が部屋で対象の Issue と動かし先を述べ、Grok Bot がその記録を Issue にコメントしたとき（Status はボードの書き込みのキューで書く） | 指揮役はボードの書き込みのキュー（runbook の「ボード Status 更新キュー」）で Status を書ける。ただし巡回は、照合できないステータスの変更を人間の操作として扱う（runbook の「ステータスの導出規則」の 3）ため、Grok Bot が人間の合図として動かした分を数える規則が要る | 文書（runbook の「ボードと巡回」）・スクリプト（`scripts/conductor-watch.sh` の人間の操作の識別） |
| H05 | 指揮役が `automerge` ラベルで Kodiak にマージを任せることを認める（マージの委任） | `docs/superpowers/specs/agent-team.md:18`、`docs/superpowers/specs/agent-team.md:22` | GitHub: 個々の操作は無い。委任の方針として仕様に書かれている | できる | PR ごとの操作は指揮役が `automerge` ラベルで行っている（runbook の「1. 役割と触ってよい範囲」）。人間に残るのは委任の方針だけ | 文書（仕様の「人間が行うのは次の 3 つだけ」の 3） |
| H06 | 実装担当の止まり（`needs-human`）に判断を出す | `docs/superpowers/specs/agent-team.md:22`、`.agents/skills/implement-issue/SKILL.md:19`、`.agents/skills/implement-issue/SKILL.md:32`、`.agents/skills/implement-issue/SKILL.md:46`、`docs/agents/grokbot-runbook.md:328` | GitHub: 止まりのコメントを Issue で読み、判断をコメントで書く（部屋で知らされる） | 条件付き（`#603`）。条件: `#603` の必ず人間に聞く範囲に当たらないとき。Grok Bot が `needs-decision` で受けて決め、目印 `<!-- grokbot-decision -->` のコメントを書く | `#603` で決めた流れ（実装担当の止まりはすべて Grok Bot に渡し、Grok Bot が自分で決めるか人間に聞くかを選ぶ） | 文書（`#603` が直す skill `implement-issue`・runbook・`triage-labels.md`）・スクリプト（`#602`） |
| H07 | Prior art first の方式を選ぶ | `AGENTS.md:11`、`.agents/skills/write-issue/SKILL.md:46`、`.agents/skills/write-issue/SKILL.md:72` | GitHub: Issue の比較のコメントを読み、方式を選んでコメントする | できない | `#603` の必ず人間に聞く範囲: Prior art first の方式の選択が挙がっている | 文書（変えるなら `#603` の必ず人間に聞く範囲と AGENTS.md の「Prior art first」） |
| H08 | `needs-human` を外す | `docs/agents/grokbot-runbook.md:15`、`docs/agents/grokbot-runbook.md:27`、`docs/agents/grokbot-runbook.md:120`、`docs/agents/grokbot-runbook.md:146`、`docs/agents/grokbot-runbook.md:324`、`docs/agents/triage-labels.md:26`、`.agents/skills/write-issue/SKILL.md:186`、`.agents/skills/write-issue/SKILL.md:187` | GitHub: Issue のラベルを外す。見張りは外した時刻の前 24 時間の hatai 本人のコメントを裏付けとして確かめる | 条件付き（`#603`）。条件: 人間が部屋で答え、Grok Bot が人間の言葉の引用・答えた時刻・目印 `<!-- grokbot-human-decision -->` を含むコメントを hatai の名義で書いた後 | `#603` で決めた流れ。見張りが確かめられるのは目印のコメントがあることだけで、人間が本当にそう言ったかは Grok の部屋の記録にしか無い（`#603` が受け入れる危険） | 文書（`#603` が直す runbook・`triage-labels.md`・skill `write-issue`）・スクリプト（`#602` のラベルの裏付け検査） |
| H09 | Issue のコメントに決定・判断を書く（未決事項の選択、分割計画、範囲の変更、方式の指示） | `.agents/skills/implement-issue/SKILL.md:21`、`.agents/skills/implement-issue/SKILL.md:67`、`.agents/skills/implement-issue/SKILL.md:244`、`.agents/skills/write-issue/SKILL.md:158` | GitHub: hatai の名義で Issue にコメントする | 条件付き。条件: `#603` の必ず人間に聞く範囲に当たらず、Grok Bot が hatai の名義で目印付きのコメントとして書くとき | 実装担当は作成者が hatai のコメントだけを決定として扱う（skill `implement-issue` の「決まり」）。Grok Bot が hatai の名義で書けるかは `#603` の未決事項と同じ確認になる | 文書（skill `implement-issue` の「決まり」、skill `write-issue` の未決事項の書き方）・権限（Grok Bot が hatai の名義で書く手段） |
| H10 | 目印のコメント（`<!-- pr-size-exception -->`・`<!-- split-plan -->`）を hatai の名義で書く | `AGENTS.md:37`、`docs/agents/triage-labels.md:27`、`.agents/skills/command-team/SKILL.md:182`、`.agents/skills/review-pr/SKILL.md:40`、`.agents/skills/review-pr/SKILL.md:63`、`.agents/skills/review-pr/SKILL.md:108` | GitHub: 人間の名義の `gh issue comment` で書く（指揮役が書くときも人間の名義の `gh` を使う） | 条件付き。条件: Grok Bot が hatai の名義の書き込み手段を持つとき。超過を認める・分割するという判断はすでに指揮役の役目 | レビュー担当は作成者が `hatai` であることで、実装担当や第三者のコメントと区別している（skill `command-team` の「マージ」）。名義を Grok Bot の別名義に変えるなら、レビュー担当の確かめ方も変わる | 権限（Grok Bot への hatai の名義の書き込み手段）。別名義にするなら文書（skill `review-pr` の手順 1〜3、`triage-labels.md`、AGENTS.md の「Agent workflow」） |
| H11 | キューが詰まったとき、または必須チェックが失敗したまま（`pr-size-exception`）の PR を admin マージする | `AGENTS.md:37`、`docs/superpowers/specs/agent-team.md:76`、`docs/superpowers/specs/agent-team.md:85`、`docs/agents/grokbot-runbook.md:21`、`docs/agents/grokbot-runbook.md:98`、`.agents/skills/command-team/SKILL.md:183`、`.agents/skills/command-team/SKILL.md:192` | GitHub: hatai の名義の `gh pr merge --admin` | できない | 権限: admin マージは GitHub App の権限に無く、`enforce_admins: false` の迂回は admin だけができる。`#603` の必ず人間に聞く範囲にも admin マージが挙がっている | 権限（Grok Bot に admin を渡す） |
| H12 | ルールセット・ブランチ保護の設定を変え、Kodiak（GitHub App）をインストールする | `docs/superpowers/specs/agent-team.md:80`、`docs/superpowers/specs/agent-team.md:87`、`docs/superpowers/specs/agent-team.md:92`、`docs/superpowers/specs/agent-team.md:95` | GitHub: リポジトリの設定画面 | できない | 権限: ルールセットと App のインストールはリポジトリの admin の操作で、`pelway-dev` のトークンはクラシックブランチ保護を読むこともできない（仕様の「マージキュー」） | GitHub の設定（admin 権限） |
| H13 | `.github/workflows/` を変えて push する、`actions.lock` を作り直す | `docs/superpowers/specs/agent-team.md:59`、`.agents/skills/implement-issue/SKILL.md:34`、`docs/agents/command-team-details.md:89` | GitHub: hatai の名義で push する。`actions.lock` は指揮役が人間の名義の `gh actions-lock` で作り直す | できない | 権限: `pelway-dev` に Workflows 権限を付けない決まりで、`gh actions-lock` は人間の名義の `gh` 拡張でしか動かない。`#603` の必ず人間に聞く範囲にも `.github/workflows/` の変更と push が挙がっている | 権限（Workflows 権限） |
| H14 | PR をレビューする・コメントする（「hatai 待ち」の依頼に応える、自分から指摘する） | `docs/superpowers/specs/agent-team.md:46`、`docs/superpowers/specs/agent-team.md:120`、`docs/agents/grokbot-runbook.md:54`、`docs/agents/grokbot-runbook.md:55`、`docs/agents/grokbot-runbook.md:69`、`docs/agents/grokbot-runbook.md:71`、`docs/agents/grokbot-runbook.md:72`、`docs/agents/grokbot-runbook.md:73`、`docs/agents/grokbot-runbook.md:201`、`docs/agents/grokbot-runbook.md:293`、`.agents/skills/command-team/SKILL.md:172`、`.agents/skills/command-team/SKILL.md:176`、`docs/agents/command-team-details.md:68`、`docs/agents/command-team-details.md:70`、`docs/agents/command-team-details.md:72`、`docs/agents/command-team-details.md:74`、`docs/agents/command-team-details.md:75`、`docs/agents/command-team-details.md:76`、`docs/agents/command-team-details.md:78`、`docs/agents/command-team-details.md:80` | GitHub: PR のレビュー・インラインコメント・PR コメント。依頼は主 PR の `[review-request-human]` のコメントで記録される | 条件付き。条件: 人間が部屋で Grok Bot に指摘を述べ、Grok Bot がその引用を PR にコメントしたとき（人間が GitHub で直接レビューする経路は残す） | 実装スレッドへの伝達はすでに指揮役が `t3 thread send` で行う（仕様の「入口A（通常の開発）の流れ」の 6）。巡回は hatai の投稿を検知の起点にしている（runbook の「hatai のレビューに関する連動」）ため、Grok Bot が記録したコメントを hatai の指摘として数える規則が要る | 文書（runbook の「hatai のレビューに関する連動」、`command-team-details.md` の「hatai のレビューの扱い」）・スクリプト（巡回の検知） |
| H15 | 指揮役のエスカレーション（停止処理、停滞、依存関係が読めない、blocker の開き直し、主 PR が閉じた、レビュー担当の不在）に判断を出す | `docs/superpowers/specs/agent-team.md:124`、`docs/agents/grokbot-runbook.md:65`、`docs/agents/grokbot-runbook.md:107`、`docs/agents/grokbot-runbook.md:115`、`docs/agents/grokbot-runbook.md:117`、`docs/agents/grokbot-runbook.md:118`、`docs/agents/grokbot-runbook.md:126`、`docs/agents/grokbot-runbook.md:292`、`docs/agents/grokbot-runbook.md:318`、`docs/agents/grokbot-runbook.md:319`、`docs/agents/grokbot-runbook.md:320`、`docs/agents/grokbot-runbook.md:322`、`docs/agents/grokbot-runbook.md:340`、`docs/agents/grokbot-runbook.md:468`、`.agents/skills/command-team/SKILL.md:205` | GitHub: `needs-human` の付いた Issue を読み、判断をコメントで書く（部屋で知らされる） | 条件付き。条件: `#603` の必ず人間に聞く範囲に当たらず、Grok Bot が判断を目印のコメントで Issue に書くとき（`#603` の流れを実装担当の止まり以外に広げる） | `#603` は実装担当の止まりと承認の待ちだけを扱い、`stalled` のエスカレーションは今のまま `needs-human` とする（`#603` の対象外） | 文書（runbook の「7. エスカレーションと停滞」と「割り当て・停止・再開」、仕様の「停止・迷走への対処」、skill `command-team` の「3. エスカレーション」） |
| H16 | 割り当ての順番を Todo の並び順から変えることを了承する | `docs/agents/grokbot-runbook.md:114` | GitHub または部屋: 開発リードを通した確認に答える。割り当てのコメントに了承の根拠のリンクが貼られる | 条件付き。条件: 依存関係（blocker）や枠の都合による入れ替えに限る。優先度そのものを変える入れ替えは人間に聞く | 順番は Todo の並び順（人間の合図）で決まる。依存関係や枠による入れ替えは並び順の意図を変えない | 文書（runbook の「割り当て・停止・再開」） |
| H17 | 承認でない入力の待ち（`waiting`）に答える | `docs/agents/grokbot-runbook.md:267` | T3: スレッドの画面で質問に答える（部屋で知らされる） | 条件付き（`#603`）。条件: `#603` の必ず人間に聞く範囲に当たらない質問のとき。Grok Bot が `t3_pending_request` で答える | `#603` で決めた流れ | 文書（`#603` が直す runbook の「5. イベントごとの対応」） |
| H18 | 権限の承認の待ち（`waiting`）を承認する | `docs/agents/grokbot-runbook.md:267` | T3: スレッドの画面で承認する（部屋で知らされる） | できない | 権限: `t3_pending_request` は権限の承認に使えない（`#603` の「T3 の道具の制約」） | 権限（T3 の道具が権限の承認を受け付けること） |
| H19 | スレッドを止める（T3 での取り消し） | `docs/agents/grokbot-runbook.md:266` | T3: スレッドの画面で止める | 条件付き。条件: 人間が部屋で止めるスレッドを名指ししたとき | スレッドの取り消しは指揮役の「しないこと」（runbook の「1. 役割と触ってよい範囲」）にある | 文書（runbook の「1. 役割と触ってよい範囲」の「しないこと」） |
| H20 | スレッドを T3 で手で立てる | `docs/agents/grokbot-runbook.md:151`、`docs/agents/grokbot-runbook.md:180`、`.agents/skills/command-team/SKILL.md:43` | T3: スレッドを手で立てる。巡回は手で立てたスレッドに介入せず、報告に一覧を書く | 条件付き。条件: チームの Issue の作業として立てるとき（個人の作業のスレッドは今のまま） | 指揮役は `t3_thread_launch` で立てられ、Grok Bot が立てれば割り当ての記録に載り巡回の対象になる | 文書（仕様の「タスクの入口と流れ」に、チームの作業のスレッドは Grok Bot に頼むと書く） |
| H21 | 代行の指揮役を指名して頼む、正本の指揮役へ戻す | `AGENTS.md:35`、`docs/superpowers/specs/agent-team.md:10`、`docs/agents/grokbot-runbook.md:444`、`docs/agents/grokbot-runbook.md:448`、`docs/agents/grokbot-runbook.md:450`、`.agents/skills/command-team/SKILL.md:43`、`.agents/skills/command-team/SKILL.md:260`、`docs/agents/command-team-details.md:99`、`docs/agents/command-team-details.md:103` | PC: 代行者のセッション（例: Claude Code）を立てて頼み、その会話が「部屋」になる | できない | PC の物理的な操作: 代行は Grok Bot が動かないときの体制で、人間がホストで代行者のセッションを立てる | なし（Grok Bot が動かないときの手段のため） |
| H22 | 代行の指揮役のスレッド ID を見張り役に登録・解除する（`pelway-agent-watch --register`・`--unregister`） | `docs/agents/grokbot-runbook.md:145`、`docs/agents/command-team-details.md:108`、`docs/agents/command-team-details.md:112`、`docs/dev-environment.md:685` | PC: ホストの端末で、T3 の画面か `t3 thread list --json` で確かめたスレッド ID を渡して実行する | できない | PC の物理的な操作: ホストの端末で実行し、登録ファイルは sandbox から書けない置き場（`~/.config/pelway-agent-notify/`）にある | なし（sandbox から書けない置き場であることが守りになっている） |
| H23 | 指揮役を立て直す（`conductor-rebuild` による恒久交代）、新指揮役の権限を T3 の画面で直す | `docs/agents/conductor-context.md:102`、`docs/agents/conductor-context.md:104`、`docs/agents/conductor-context.md:109`、`docs/dev-environment.md:62` | PC: ホストの本体 checkout で、blob の sha を GitHub の main と比べてから実行する。T3: 新指揮役の権限を直す | できない | PC の物理的な操作: ホストの本体 checkout で実行する決まりで、worktree からは止まる。比べる手順は sandbox から書き換えられた版を実行しないための守り | なし（ホストで実行することが守りになっている） |
| H24 | コンテキスト圧縮・交代が進められないときに確認する | `docs/agents/conductor-context.md:33`、`docs/agents/conductor-context.md:99`、`docs/agents/conductor-context.md:119`、`docs/dev-environment.md:583` | 部屋: 指揮役から確認を求められて答える | 条件付き。条件: 対象が代行の指揮役で、Grok Bot が動いているとき（Grok Bot 自身の圧縮・交代は Grok Bot が確かめられない） | 確認の中身は外部化と引き継ぎの照合で、正本の指揮役が読める GitHub と巡回記録にある | 文書（`conductor-context.md` の「トリガー（暫定の観測基準）」の打ち切り、「恒久交代」、「導入条件」） |
| H25 | 見直し・改善を決める（圧縮の暫定数値の見直し、試行の輸入スキルを通常運用に移すか、正本の変更が要る候補を Issue にするか） | `docs/agents/conductor-context.md:125`、`.agents/skills/implement-issue/SKILL.md:365`、`.agents/skills/investigate-bug/SKILL.md:191`、`.agents/skills/command-team/SKILL.md:256`、`docs/agents/grokbot-runbook.md:486` | GitHub: 改善の Issue を起票して承認する | 条件付き。条件: Grok Bot が根拠（巡回記録・worklog）を書いた Issue を起票し、その Issue を H02 の経路で承認するとき | 見直しは正本の変更を伴うので、起票の承認と同じ門を通す | 文書（`conductor-context.md` の「観測と見直し」、skill `implement-issue` の「輸入スキル」と skill `investigate-bug` の「輸入スキル（mattpocock/skills）」、skill `command-team` の「定期キュレーション（worklog の走査）」） |
| H26 | PR を閉じる・作り直す（長命 PR の棚卸しの結果、Dependabot の PR） | `docs/agents/command-team-details.md:22`、`docs/agents/command-team-details.md:88` | GitHub: PR を閉じる（棚卸し担当・指揮役が PR に依頼のコメントを書き、部屋で人間に知らせるのは指揮役） | 条件付き。条件: 人間が部屋で閉じる PR を名指しし、棚卸し担当または Grok Bot がその記録を PR にコメントしたとき | PR のクローズは指揮役の「しないこと」（runbook の「1. 役割と触ってよい範囲」）にある | 文書（runbook の「1. 役割と触ってよい範囲」、`command-team-details.md` の「週次: 長命 PR の棚卸し」と「Dependabot の PR の扱い」） |
| H27 | 外部のエージェント（例: Muse）にレビュースレッドの投入を許す | `docs/agents/command-team-details.md:30` | 部屋: 投入してよい外部のエージェントを許可する | できない | `#603` の必ず人間に聞く範囲: 外部のエージェントにリポジトリの作業を許すことは、秘密・外部への接続に当たる | 文書（変えるなら `#603` の必ず人間に聞く範囲） |
| H28 | 「Devin CLI のハーネスごと任せる」ことを Issue で明示する | `docs/superpowers/specs/agent-team.md:66`、`.agents/skills/command-team/SKILL.md:115` | GitHub: Issue の本文に書く | できる | 同じ振り分けは指揮役の判断でもできる（skill `command-team` の「実装のモデルと effort」の表） | 文書（人間の明示を部屋での依頼として Grok Bot が Issue に書くと、仕様と skill `command-team` に足す） |
| H29 | `ready-for-human` の Issue を実装する | `docs/agents/triage-labels.md:10`、`docs/agents/command-team-details.md:21` | GitHub と PC: 人間が自分で実装する | できない | 権限または PC の物理的な操作: ラベルの定義が人間の実装を求めるもので、エージェントに渡せない理由（権限か PC の物理的な操作か）は Issue ごとに違う | なし（ラベルの定義のため） |
| H30 | ボードを読み書きする名義（人間の名義の `gh` の `read:project`・`project` スコープ）を用意する | `docs/superpowers/specs/agent-team.md:62`、`docs/agents/grokbot-runbook.md:17`、`docs/agents/grokbot-runbook.md:33`、`docs/agents/grokbot-runbook.md:130` | PC: ホストの `gh` にスコープを付ける。ホストの `conductor-watch.sh` が人間の名義でボードの書き込みのキューを処理する | できない | 権限: ボードの読み書きは人間の名義の `project` スコープで行う決まりで、App の名義の経路は無い。PC の物理的な操作: スコープはホストの `gh` の認証で付ける | 権限（App の名義でボードを読み書きする経路） |
| H31 | 指揮役の scheduled task の間隔を設定する | `docs/agents/grokbot-runbook.md:140` | T3: scheduled task の設定 | 条件付き。条件: Grok Bot が T3 Connect の MCP で scheduled task を設定できるとき（T3 の MCP には `schedule_task` がある） | 設定は T3 の中の操作で、ホストの操作を伴わない | 文書（runbook の「指揮役の定期巡回（スケジュール起床）」の「scheduled task の間隔の設定は hatai が行う」） |
| H32 | 終わった worktree の残りを整理する（`mise run agent-worktree-prune`） | `docs/superpowers/specs/agent-team.md:48`、`docs/agents/grokbot-runbook.md:372` | PC: ホストで実行する | できない | PC の物理的な操作: ホストで実行し、sandbox の外の worktree を消す。`#603` の必ず人間に聞く範囲にも sandbox の外での実行が挙がっている | なし（sandbox の外での削除のため） |
| H33 | PC の自動起動を設定する（pitchfork の login 時の起動、本体 checkout の namespace の登録、supervisor の PATH）、止まっていたときに状態ファイルを退避して起動し直す | `docs/dev-environment.md:692`、`docs/dev-environment.md:698`、`docs/dev-environment.md:705` | PC: `pitchfork boot enable`、`~/.config/pitchfork/config.toml` の編集、状態ファイルの改名と `pitchfork start agent-watch`。supervisor の PATH（systemd のユーザーユニット）は対象のファイルに書かれておらず、`docs/worklogs/bugs/agent-watch-not-running/report.md` の 67 行目にある | できない | PC の物理的な操作: ホストのユーザー設定と systemd のユーザーユニットを変える | なし（ホストの設定のため） |
| H34 | codex の requirements 層とレビュー担当の sandbox の設定をホストに適用する（`/etc/codex/requirements.toml`、`~/.config/devin-review/config.json`、`~/.claude-review/settings.json`） | `docs/dev-environment.md:197`、`docs/dev-environment.md:317`、`docs/dev-environment.md:335` | PC: root 権限とホストの設定ファイルの編集 | できない | PC の物理的な操作: root 権限とホストの設定ファイルで、sandbox からは書けない。`#603` の必ず人間に聞く範囲にも秘密・外部への接続が挙がっている | なし（sandbox の外の設定のため） |
| H35 | レビュー用 Claude にログインし、T3 に `claude_review` を足す | `docs/dev-environment.md:234`、`docs/dev-environment.md:263` | PC: `CLAUDE_CONFIG_DIR=~/.claude-review claude` で対話のログイン。T3: プロバイダの設定 | できない | PC の物理的な操作: 対話のログインとホストの T3 の設定 | なし（対話のログインのため） |
| H36 | QA 用の GitHub App `pelway-qa` を作り、インストールし、秘密鍵を登録する（戻すときはインストールを解除して消す） | `docs/dev-environment.md:437`、`docs/dev-environment.md:441`、`docs/dev-environment.md:467` | GitHub: App の設定画面。PC: `~/.config/pelway-agent-qa/fnox.toml` に登録する | できない | 権限: App の作成とインストールはアカウントの設定画面の操作。PC の物理的な操作: 秘密鍵をホストの fnox に登録する | なし（App の作成と秘密鍵の扱いのため） |
| H37 | 安全と方針を決裁する（残るリスクの受容、ネットワークの方針、ライセンスの許可、LLM を呼ぶ根拠の既定値、レビュー担当の使用停止） | `docs/superpowers/specs/agent-team.md:152`、`docs/agents/conductor-context.md:109`、`docs/dev-environment.md:270`、`docs/dev-environment.md:416`、`docs/dev-environment.md:420`、`docs/dev-environment.md:492`、`docs/dev-environment.md:499`、`docs/dev-environment.md:604`、`docs/dev-environment.md:615` | GitHub: Issue・PR のコメントで決める。決定は文書に記録される | 条件付き。条件: `#603` の必ず人間に聞く範囲（sandbox の外での実行、秘密・外部への接続）に当たらない決裁に限る（ライセンスの許可、既定値など） | リスクの受容とネットワークの方針は、秘密・外部への接続か sandbox の外での実行に当たる | 文書（決裁の記録の形を目印のコメントに揃える） |
| H38 | 脆弱性の疑いで止まった調査に判断を出す（公開してよい範囲、証拠と修正案の置き場） | `.agents/skills/investigate-bug/SKILL.md:119`、`.agents/skills/investigate-bug/SKILL.md:127`、`.agents/skills/investigate-bug/SKILL.md:130`、`.agents/skills/investigate-bug/SKILL.md:133`、`.agents/skills/investigate-bug/SKILL.md:134`、`docs/agents/command-team-details.md:66`、`docs/agents/grokbot-runbook.md:285` | 部屋と GitHub: スレッドの返答の概要を読み、判断を伝える。Issue なら `needs-human` が付く | できない | `#603` の必ず人間に聞く範囲: 秘密・外部への接続に当たり、`#603` も skill `investigate-bug` の止まりを今のまま `needs-human` とする（`#603` の対象外） | 文書（変えるなら `#603` の必ず人間に聞く範囲と skill `investigate-bug` の手順 4） |
| H39 | 代行中の指揮役が止まったときに判断する（受け入れ条件と矛盾する操作、main の fast-forward ができない） | `.agents/skills/command-team/SKILL.md:86`、`.agents/skills/command-team/SKILL.md:191` | 部屋: 代行者の報告を受けて判断する | 条件付き。条件: Grok Bot が復旧していて、代行者の報告を受けられるとき（代行は Grok Bot が動かないときの体制） | 報告先が hatai なのは、代行中は正本の指揮役が動いていないため | 文書（skill `command-team` の「手当て・引き継ぎの前の確認」と「マージ」の「代行の場合は hatai」） |

## 操作ではない行

上の判定の決まりで操作ではないとした行。ファイルごとに並べる。

- `AGENTS.md`: `AGENTS.md:48`、`AGENTS.md:52`
- `docs/superpowers/specs/agent-team.md`: `docs/superpowers/specs/agent-team.md:3`、`docs/superpowers/specs/agent-team.md:9`、`docs/superpowers/specs/agent-team.md:74`、`docs/superpowers/specs/agent-team.md:82`、`docs/superpowers/specs/agent-team.md:100`、`docs/superpowers/specs/agent-team.md:102`
- `docs/agents/grokbot-runbook.md`: `docs/agents/grokbot-runbook.md:10`、`docs/agents/grokbot-runbook.md:45`、`docs/agents/grokbot-runbook.md:52`、`docs/agents/grokbot-runbook.md:105`、`docs/agents/grokbot-runbook.md:108`、`docs/agents/grokbot-runbook.md:124`、`docs/agents/grokbot-runbook.md:125`、`docs/agents/grokbot-runbook.md:132`、`docs/agents/grokbot-runbook.md:166`、`docs/agents/grokbot-runbook.md:170`、`docs/agents/grokbot-runbook.md:172`、`docs/agents/grokbot-runbook.md:176`、`docs/agents/grokbot-runbook.md:296`、`docs/agents/grokbot-runbook.md:353`、`docs/agents/grokbot-runbook.md:354`、`docs/agents/grokbot-runbook.md:355`、`docs/agents/grokbot-runbook.md:356`、`docs/agents/grokbot-runbook.md:357`、`docs/agents/grokbot-runbook.md:370`、`docs/agents/grokbot-runbook.md:391`、`docs/agents/grokbot-runbook.md:393`、`docs/agents/grokbot-runbook.md:398`、`docs/agents/grokbot-runbook.md:412`、`docs/agents/grokbot-runbook.md:433`、`docs/agents/grokbot-runbook.md:435`、`docs/agents/grokbot-runbook.md:451`、`docs/agents/grokbot-runbook.md:461`、`docs/agents/grokbot-runbook.md:465`
- `docs/agents/command-team-details.md`: `docs/agents/command-team-details.md:32`、`docs/agents/command-team-details.md:106`、`docs/agents/command-team-details.md:115`
- `docs/agents/triage-labels.md`: `docs/agents/triage-labels.md:24`、`docs/agents/triage-labels.md:25`
- `docs/dev-environment.md`: `docs/dev-environment.md:231`、`docs/dev-environment.md:235`、`docs/dev-environment.md:283`、`docs/dev-environment.md:284`、`docs/dev-environment.md:285`、`docs/dev-environment.md:306`、`docs/dev-environment.md:340`、`docs/dev-environment.md:341`、`docs/dev-environment.md:342`、`docs/dev-environment.md:343`、`docs/dev-environment.md:352`、`docs/dev-environment.md:372`、`docs/dev-environment.md:433`、`docs/dev-environment.md:449`、`docs/dev-environment.md:505`、`docs/dev-environment.md:690`
- `.agents/skills/command-team/SKILL.md`: `.agents/skills/command-team/SKILL.md:16`、`.agents/skills/command-team/SKILL.md:66`、`.agents/skills/command-team/SKILL.md:68`、`.agents/skills/command-team/SKILL.md:70`、`.agents/skills/command-team/SKILL.md:72`、`.agents/skills/command-team/SKILL.md:73`、`.agents/skills/command-team/SKILL.md:75`、`.agents/skills/command-team/SKILL.md:134`、`.agents/skills/command-team/SKILL.md:148`、`.agents/skills/command-team/SKILL.md:157`、`.agents/skills/command-team/SKILL.md:180`、`.agents/skills/command-team/SKILL.md:190`
- `.agents/skills/write-issue/SKILL.md`: `.agents/skills/write-issue/SKILL.md:103`
- `.agents/skills/implement-issue/SKILL.md`: `.agents/skills/implement-issue/SKILL.md:13`、`.agents/skills/implement-issue/SKILL.md:27`、`.agents/skills/implement-issue/SKILL.md:59`、`.agents/skills/implement-issue/SKILL.md:71`、`.agents/skills/implement-issue/SKILL.md:222`、`.agents/skills/implement-issue/SKILL.md:240`、`.agents/skills/implement-issue/SKILL.md:306`、`.agents/skills/implement-issue/SKILL.md:331`、`.agents/skills/implement-issue/SKILL.md:390`
- `.agents/skills/investigate-bug/SKILL.md`: `.agents/skills/investigate-bug/SKILL.md:21`、`.agents/skills/investigate-bug/SKILL.md:212`
- `.agents/skills/review-pr/SKILL.md`: `.agents/skills/review-pr/SKILL.md:47`

## hatai が決めること

提案が「できる」か「条件付き」の行ごとに、経路を変えるかを hatai が「選ぶ」の欄に書く（変える・変えない・保留のどれか）。変えると選んだ行は、行ごとに別の Issue にする。

| ID | 人間がすること | 提案 | 選ぶ |
|---|---|---|---|
| H01 | Issue を起票する（Issue Form `agent-task`、skill `write-issue`） | できる |  |
| H02 | 起票した Issue に `ready-for-agent` を付ける（入場の承認） | 条件付き |  |
| H03 | 調査の報告の後、指揮役が起票した Issue を `ready-for-agent` で承認する | 条件付き |  |
| H04 | ボードのカードを動かす（Backlog から Todo へ動かして処理の合図、Backlog へ戻して停止、Done へ動かして閉じる）、Issue を直接閉じる、着手後に `ready-for-agent` を外す | 条件付き |  |
| H05 | 指揮役が `automerge` ラベルで Kodiak にマージを任せることを認める（マージの委任） | できる |  |
| H06 | 実装担当の止まり（`needs-human`）に判断を出す | 条件付き（`#603`） |  |
| H08 | `needs-human` を外す | 条件付き（`#603`） |  |
| H09 | Issue のコメントに決定・判断を書く（未決事項の選択、分割計画、範囲の変更、方式の指示） | 条件付き |  |
| H10 | 目印のコメント（`<!-- pr-size-exception -->`・`<!-- split-plan -->`）を hatai の名義で書く | 条件付き |  |
| H14 | PR をレビューする・コメントする（「hatai 待ち」の依頼に応える、自分から指摘する） | 条件付き |  |
| H15 | 指揮役のエスカレーション（停止処理、停滞、依存関係が読めない、blocker の開き直し、主 PR が閉じた、レビュー担当の不在）に判断を出す | 条件付き |  |
| H16 | 割り当ての順番を Todo の並び順から変えることを了承する | 条件付き |  |
| H17 | 承認でない入力の待ち（`waiting`）に答える | 条件付き（`#603`） |  |
| H19 | スレッドを止める（T3 での取り消し） | 条件付き |  |
| H20 | スレッドを T3 で手で立てる | 条件付き |  |
| H24 | コンテキスト圧縮・交代が進められないときに確認する | 条件付き |  |
| H25 | 見直し・改善を決める（圧縮の暫定数値の見直し、試行の輸入スキルを通常運用に移すか、正本の変更が要る候補を Issue にするか） | 条件付き |  |
| H26 | PR を閉じる・作り直す（長命 PR の棚卸しの結果、Dependabot の PR） | 条件付き |  |
| H28 | 「Devin CLI のハーネスごと任せる」ことを Issue で明示する | できる |  |
| H31 | 指揮役の scheduled task の間隔を設定する | 条件付き |  |
| H37 | 安全と方針を決裁する（残るリスクの受容、ネットワークの方針、ライセンスの許可、LLM を呼ぶ根拠の既定値、レビュー担当の使用停止） | 条件付き |  |
| H39 | 代行中の指揮役が止まったときに判断する（受け入れ条件と矛盾する操作、main の fast-forward ができない） | 条件付き |  |
