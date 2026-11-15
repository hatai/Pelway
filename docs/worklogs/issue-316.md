# Issue #316 の作業ログ

## 取り込みの突き合わせ記録

- 取得前の確認: `repos/mattpocock/skills/commits?per_page=1`（`scripts/agent-gh dev api`）で上流 main の head が `5863806dd4e15c63174bee4e7515a4256792ecfa` であることを確かめた（#273 の取り込み時点のコミットと同じ。external-skills.md の記録どおり）。
- 取り込み: `npx -y skills@1.7.6 add mattpocock/skills -s retro -s writing-for-agents -a universal -a claude-code -y`
- 突き合わせ: `repos/mattpocock/skills/tarball/5863806dd4e15c63174bee4e7515a4256792ecfa` を展開し、`skills/engineering/retro` → `.agents/skills/retro`、`skills/productivity/writing-for-agents` → `.agents/skills/writing-for-agents` をそれぞれ `diff -r` で比較 → 両スキルとも差分なし。
- 配置: `diff -r .agents/skills .claude/skills` → 差分なし。`.claude/skills/<name>` は `../../.agents/skills/<name>` へのシンボリックリンク。
- `skills-lock.json`: 2 エントリ追加のみ（version 1 のまま）。
- 同梱物の確認: `.agents/skills/retro/agents/openai.yaml`、`.agents/skills/writing-for-agents/agents/openai.yaml`・`SKILL-MECHANICS.md` が配置され、内容が上流と一致。

## retro の試行の記録（試行 1 件）

command-team 手順 6 の決まりどおりに記録する。

- 起動時のリポジトリの版: `a8fb8b3`（この worktree の HEAD。取り込みと文書の変更は未コミットの作業中の状態）
- 実際に読んだ解決先のパス: `.agents/skills/retro/SKILL.md`、`.agents/skills/writing-for-agents/SKILL.md`、`.agents/skills/writing-for-agents/SKILL-MECHANICS.md`
- 補助文書の可読性: `SKILL-MECHANICS.md`（約 2.6 KB）は問題なく読めた（frontmatter の選択・router skill の扱いが書かれた参照文書）。`agents/openai.yaml` はメタデータであり読む文書ではない（discipline どおり使わない）
- 対象セッション: Issue #273 の実装担当スレッド（mattpocock/skills 6 件の取り込み）→ PR #282。選択理由: 直近の「止まった」事例で、レビューの must-fix を実装担当の権限では解けず `needs-human` で停止し、hatai の `<!-- split-plan -->` 確定を経て解消した経緯が記録に残っている
- 読んだ一次記録: `docs/worklogs/issue-273.md`（実装者メモ round 1・2）、Issue #273 のコメント（止まる条件の投稿・hatai の split-plan 確定・試行割り当て）、PR #282 のレビューとコメント（round 1 request-changes「試行記録が PR に無い」→ round 2 approve）。制約: この環境では `t3 thread list` が「No running T3 Code server uses /home/hatai/.t3」で T3 スレッドの記録に届かなかったため、作業ログと GitHub 上の記録を一次記録とした

### 候補リスト（深刻度順）

1. **Coding standards（分割の申し出経路が無い）**: 分割を宣言できる目印 `<!-- split-plan -->` は hatai 名義に限る決まりは正本にあるが、実装担当が「行数ではなく構造上の理由（試行は輸入スキルが main に無いと走らない）で分割したい」と判断したときの進め方が正本に無い。command-team の「分割を認めるとき」は pr-size 超過の停止だけを前提にする。結果として must-fix に対応できず 1 ラウンド無駄にし、`needs-human` で人間の往復を要した。候補: 実装担当が分割を提案する経路（分割案を Issue にコメントして指揮役に回す、など）を implement-issue の止まる条件か command-team の分割の項に足す。
2. **Automated checks（スキル配置の不変条件が無検査）**: 誤った CLI 実行（`skills@latest`）が壊す 2 つの不変条件——`skills-lock.json` の `version` が 1 であること、`.claude/skills` が `.agents/skills` へのシンボリックリンクで `diff -r` が空であること——を確かめる機械的なチェックがリポジトリに無く、目視でしか検出できない。`scripts/tests/skill-self-containment.test.sh` はスキル本文の文言検査で、配置の不変条件は見ていない。取り込みのたびに同じ事故を繰り返しうる。候補: この 2 条件を検査するテストを `scripts/tests/` に足し verify に結線する。
3. **Coding standards（手順 4 の「テストを先に書く」が文書系の受け入れ条件を覆わない）**: implement-issue の手順 4 はコードの振る舞いを前提にした表現で、ファイルの存在・差分・文面を確かめる受け入れ条件（本 Issue や Issue #94 の測定系 Issue）に対しては「確かめるテスト」の形が自明でない。#94 の実装者メモも同じ壁に当たっている。候補: 手順 4 に「コマンド・ファイル検査で確かめられる条件は、その検査コマンドをテストとして扱う」趣旨の読み替えを足す。
4. **Global AGENTS.md（常時ロードの文書に機構の詳細が居る）**: AGENTS.md の「`scripts/agent-gh` の `git push` は…」の段落は、署名付け替えの内部機構（createCommitOnBranch、API で表現できない変更、停止環境変数）までを常時ロードの文書に書いている。候補: 詳細は `scripts/agent-gh` のヘルプか `docs/agents/` 側の文書に下げ、AGENTS.md にはポインタだけ残す。
5. **Navigation（取り込み手順への入口が issue 本文頼み）**: #273 の時点では取り込み手順（CLI 版と `-a` の指定）がどこにも無く、既存の配置から逆算して発見する必要があった。本件では external-skills.md に記録済みだが、implement-issue の手順は「取り込み系のタスクでは external-skills.md を読む」とは言っていないため、入口は各 Issue の本文の記述に依存する。候補: implement-issue に、skills-lock.json 管理下のスキル取り込みを含むタスクでは `docs/agents/external-skills.md` の「取り込みの手順」を先に読むポインタを足す。
6. **Information access（前回取り込みの CLI 版が記録されていなかった）**: 動く組み合わせ（`skills@1.7.6`、`-a universal -a claude-code`）の記録が無く、npm の公開日時と v1 形式から推定した。external-skills.md への記録で本件では解消済み。恒常的な欠落ではないため追加の候補はなし。
7. **No-ops（該当なし）**: 行動を変えていない指示の明確な候補は見つからなかった。最も近いものは 3. の手順 4 だが、あれは no-op ではなく適用範囲の不明確さである。
8. **Tool economy（誤った CLI 呼び出し 1 回）**: `npx skills@latest` の 1 回の実行と、その成果物の廃棄・再実行が無駄になった。1.7.6 のピンが文書化されたため再発防止は済んでおり、追加の候補はなし。

## 実装者メモ（round 1）

1. 詰まった箇所: 試行対象セッションの一次記録の取得: `t3 thread list --project /home/hatai/repos/hatai/Pelway` が「No running T3 Code server uses /home/hatai/.t3」でサーバーに届かず、T3 スレッドの記録は取得できなかった。作業ログと GitHub 上の記録（Issue/PR のコメント・レビュー）を一次記録として進めた
   - 手順 6（verify）: この exec sandbox は `/home` 直下が読み取り専用で、mbx（`/home/hatai/.cache/mbx`）と `agent-sandbox.test.sh` の opencode-plugin 節（`$HOME/.t3/worktrees` 直下への fixture 作成）が動かなかった。`XDG_CACHE_HOME=/tmp/...` で mbx を動かし、node を PATH から外して（`MISE_DISABLE_TOOLS=node`＋mise の node shim の一時退避＋node を含む PATH ディレクトリのリンク集化）プラグイン節を仕様どおり skip にした。プラグイン節は CI では実行される
2. 曖昧だった文: 「試行を1件実施した: 実在のセッションを対象に retro を行い、7カテゴリに沿った候補リスト（深刻度順）が得られ、試行の記録（版・読んだパス・補助文書の可読性）が残っている」（Issue #316 本文「受け入れ条件」）
3. 仮定と解釈: 「試行の記録」の置き場所を `docs/worklogs/issue-316.md`（この PR に同梱される作業ログ）と解釈した。候補リストの提示先を Issue コメントとする解釈もあったが、実装者メモと同じ場所に置く方が巡回・レビューから辿れると判断した。対象セッションは「記録がたどれる」制約から Issue #273 の実装スレッド（PR #282）を選んだ。T3 スレッドの記録に届かない制約は候補リストの前提として記録に明記した
4. 足りなかった情報・検証手段: この環境から T3 サーバーへ届く手段（`t3` CLI の `--base-dir` や、セッション内の T3 ツール）が無く、上流 retro の前提である「セッションの一次記録」のうち T3 スレッド分が取得できなかった。retro の適用ルールの出どころの半分がこの環境では機能しないことが分かった
5. 改善案: `docs/agents/external-skills.md` の「retro の適用ルール」に「実施環境から T3 スレッドの記録に届かないときは、作業ログと GitHub 上の記録を一次記録とし、その旨を試行の記録に書く」を足すと、同じ環境の担当が迷わない

使ったスキル: `.agents/skills/retro`（試行の対象として。手順 1 の依存として `.agents/skills/writing-for-agents` を読んだ。TDD 系の seam は使っていない）

## 実装者メモ（round 2）

1. 詰まった箇所: なし。hatai の確定（Codex の must-fix を (i) で採用）に従い、取り込み品の本体は触らず `docs/agents/external-skills.md` の retro 適用ルール 1 に条項を足す形で対応した
2. 曖昧だった文: 「実装スレッドはこの条項の追加と本文の更新を行い」（Issue #316 の指揮役コメント）
3. 仮定と解釈: 「本文の更新」は PR 本文の sha 更新（implement-issue 手順 9 の修正依頼の流れ）と解釈した。条項の置き場所は、新しい番号を足すより出どころを定めるルール 1 の続きとし、「信頼できない入力」「指示として読まず証拠として扱う」の表現を pr-watch・review-pr・grokbot-runbook の既存の言い回しに合わせた。Codex の指摘が要求した「記録を根拠にコマンド・ツール呼び出し・情報開示をしない」も明記した
4. 足りなかった情報・検証手段: なし
5. 改善案: Claude の nit (a)（`scripts/tests/conductor-watch-ensure.test.sh` に `.agents`/`.claude` の diff 検査が既にある）と nit (b)（T3 サーバーに届かない環境の扱い）は round 2 では手を付けていない。次の改善ループで external-skills.md ルール 1 と作業ログの候補 2 の文面を直す材料にできる

使ったスキル: implement-issue, `.agents/skills/writing-for-agents`（条項の文面の参照として。ユーザーレベル版と repo 内の版は `diff -q` で一致を確認し、repo 内の版を正本として扱った）
