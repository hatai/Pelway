# Issue #345 の作業ログ

## 実装者メモ（round 1）

対象 head: e0c391f68e8bfb232195a0de183b4032244adfdc（この round の実装の実体を含むコミット。agent-gh-resign による署名付け替え後の SHA）

実施者: swe-2-high（T3 Code、hatai からの直接指示のセッション）。実施者・モデル・effort の記録は PR 本文の Implementer: 行にも記載。

### 変更 ↔ 一次記録の引用 ↔ 期待する変化の対応表

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| 「週次: 長命 PR の棚卸し」を `docs/agents/command-team-details.md` へ移動（SKILL.md には読む条件つき参照） | Issue #345「肥大化の根拠」: 「週次: 長命 PR の棚卸しは週次巡回のときだけ必要」 | 毎巡回で読む本文が減り、週次巡回時だけ詳細文書を開く形になる |
| 「外部からのレビュースレッド投入」を details へ移動 | 同上: 「外部レビュー投入はレビュー滞留時だけ必要」 | 同上 |
| 「調査の担当と effort」「調査の引き渡し」を「調査（入口B）」として details へ移動（入口B の Assigned テンプレートも同節へ） | 同上: 「調査スレッドが絡むときだけ必要」 | 同上 |
| 「hatai のレビューの扱い」の hatai 待ち・伝達を details へ移動（「有効な Approve」の定義はマージ判断に毎回使うため SKILL.md に残置） | 同上: 「hatai のレビューが絡むときだけ必要」 | 同上 |
| 「マージ」の Dependabot 3 項を details の「Dependabot の PR の扱い」へ移動 | 同上: 「Dependabot PR のときだけ必要」 | 同上 |
| 「レビュー担当と effort」の「段階移行」を details へ移動（SKILL.md に発動条件つき参照） | 同上: 「抜き取り集計の時点だけ必要」 | 同上 |
| 「5. 代行の開始と終了」の開始・終了チェックを details へ移動（節名は runbook「代行」が「手順 5」として参照するため SKILL.md に残置） | `docs/agents/grokbot-runbook.md`「11. 代行」: 「開始と終了のチェックは skill `command-team` の手順 5」 | 代行セッション時だけ詳細を開く形になる |
| 「7. コンテキスト管理」の本文（約 110 行）を `docs/agents/conductor-context.md` へ移動（SKILL.md にトリガーの要約・常時ルール・正本参照を残置） | Issue #345「肥大化の根拠」: 「コンテキスト管理は圧縮トリガー・予期しない圧縮・恒久交代のときだけ読む手順」 | 最大の行数削減分。常時必要な「操作発行時の記録」の要約は本文に残る |
| 「6. 輸入スキルの試行」の試行の割り当てと grill 試行を `docs/agents/external-skills.md` の「試行の割り当て」「grill 試行」へ移動。「取り込みの discipline」は external-skills.md 側が正本のため SKILL.md 側の重複を削除 | Issue #345: 「輸入スキルの試行は試行としての割り当てのときだけ必要」 | 試行割り当ての時だけ外部スキル文書を開く。discipline の 2 箇所管理を解消 |
| 「マージ」内で重複していた「Approve の後に push があった PR の確認（rev-list が 0 件）」を 1 箇所に一本化 | Issue #345: 「重複: …が「マージ」内に 2 箇所書かれている」 | 同じ規則の記述箇所が 1 つになり、将来の変更で食い違わない |
| 「Devin CLI のレビュー担当は…確認が済むまで使わない」を「確認済み。確認記録は docs/dev-environment.md…記録が消えた場合は…」へ更新 | `docs/dev-environment.md`「レビュー担当の Devin CLI」記録欄に 2026-11-14 の確認結果が記入済み（Claude 側は既に「確認済み」の形） | 前提が消えた死文が現在の状態を表す文になる |
| 参照の向き先の更新（agent-team.md 2 箇所、runbook 3 箇所、dependabot.yml 1 箇所） | 移動した節名が SKILL.md に無くなったため、参照を正本へ直す（参照先の実在確認は Issue #345 の受け入れ条件） | 切れた参照が無い |
| 文書整合テスト 3 本の参照先更新（conductor-compaction-docs→conductor-context.md、board-patrol-docs cond3→command-team-details.md、skill-self-containment cond6c→external-skills.md） | 各テスト冒頭: 「文言を変えるときはこのテストと同じ PR で直す」 | 文言の検査対象が正本の新しい置き場所を指し、退行も検出できる |

### 詰まった箇所

1. `exec` で `sed -i 's/"\$CT" \\/"$CC" \\/g'` が 2 回続けて無変換で終わり、3 回目はクォート処理で `$CC` が空文字に展開されテストを壊した。スクリプトファイル化（`/tmp/fix-test.sh` を `bash` で実行）で確実に直した。原因はこの環境の exec 引数経由のエスケープ処理と思われる（issue-308 の `\uXXXX` 障害と同系統かは未確認・想定）。
2. `git commit -m "$(cat <<'EOF' …)"` が dash 環境で構文エラー。`-F <ファイル>` で回避。

### 曖昧だった文

- Issue #345 受け入れ条件の「`.agents/skills/command-team/SKILL.md` が 330 行未満」: トリガー要約を SKILL.md 側に残す設計を取った（正本の詳細は conductor-context.md）。「読む条件つき参照」に含める要約の粒度（トリガー値の併記の可否）は書かれていない。指揮役が「いつ doc を開くか」を本文だけで判定できるようトリガー値は残し、数値・起算点の定義は正本に寄せた。

### 仮定と解釈

- 「有効な Approve」の定義は「マージ」の必須確認として毎回必要と判断し SKILL.md に残した。hatai 待ち・伝達の手順は条件つき詳細として移した。
- 調査スレッド用の Assigned テンプレートは「スレッドを立てる指示の 2 行目」の規則の一部だが、調査（入口B）のときだけ必要なため details の「調査（入口B）」に置き、決まりの箇条に参照を残した。
- 「操作発行時の記録（常時ルール）」は圧縮と無関係に毎操作で必要と判断し、要約を SKILL.md §7 に残した（「正本の」という枕詞つき）。

### 足りなかった情報・検証手段

- `mise run script-test` は `agent-sandbox.test.sh` で失敗（`/home` が read-only で `~/.t3/worktrees` 兄弟の mkdir 不可）。issue-296/315 のメモどおりの環境制約で本変更とは無関係。ほかの文書整合テスト 6 本はすべて pass。`agent-sandbox` の検査は CI に委ねる。
- `mise run verify` は上記の script-test 失敗でローカル完走しないため、CI の実行結果に委ねる。

### 改善案

- Issue #345 の実施で、セクション移動に伴う「文言固定テストの参照先更新」が必須作業になることが分かった。improve-skills の「最適化する」に「移した文言を grep で固定している `scripts/tests/` のテストは、参照先を正本へ向け直して同じ PR に入れる」の一工程を足すと抜け漏れを防げる（改善ループへ提案）。
