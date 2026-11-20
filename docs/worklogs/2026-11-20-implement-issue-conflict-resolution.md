# implement-issue skill にマージ衝突解消の最小手順を足す（2026-11-20）

出どころ: 人間からの直接依頼（T3 セッション `$find-skills マージ衝突の解消をカバーするスキル` に続く指示）。エコシステム検索（`npx skills find merge conflict`）で候補を比較したうえで、人間が「自前の最小手順として implement-issue に書く」方式（下記の Option A）を選んだ。Issue を経由していない判断は人間の選択。

## 先行事例の比較（採用・不採用）

| 候補 | 判断 | 採ったもの / 捨てたもの |
|---|---|---|
| mattpocock/skills@resolving-merge-conflicts（551.2K インストール） | 不採用（取り込み）。上流で v1.3.0 でアーカイブ済み（「専用スキル無しで解消できる」が削除理由） | 採った: 各 hunk の解消前に両側の意図を一次情報（コミット・PR・Issue）まで遡る原則、`--ours`/`--theirs` や見た目での削除で解消しない原則、検証を通してからコミットする原則。捨てた: スキル本体（アーカイブ済みであること、周辺の流れ（メモ round・sha 更新・レビュー再委託）は Pelway 固有で自前にしか存在しないこと） |
| warpdotdev/common-skills@resolve-merge-conflicts（29.5K） | 不採用 | `git checkout --ours/--theirs` の側の丸ごと採用を推奨している点が Pelway の実績（下記）と逆。同梱 Python スクリプトも輸入規律（同梱スクリプトは実行しない）と衝突 |
| cursor/plugins@fix-merge-conflicts（2.9K、ユーザーレベルに同名版が既存） | 不採用 | 汎用チェックリストで追加価値が薄い。ユーザーレベル版の直接参照は規律上不可（輸入スキルは repo 内の明示パスで読む） |
| **A: 自前の最小手順（採用）** | 採用 | 上記 mattpocock の原則 3 点を 3 行に圧縮して implement-issue 手順 9 に書く。周辺の流れは既存の手順（5・6・8）への参照で済ませる |

## 根拠の対応表

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| 手順 9 に「修正依頼（CONFLICTING の解消など、レビュー結果を介さない修正指示）のとき」の節を 3 行で追加 | CONFLICTING 解消の修正依頼が 3 回繰り返されている: PR #388（`docs/worklogs/2026-11-16-conductor-substitute.md` の PR #388 の記録）、PR #397（`docs/worklogs/2026-11-17-conductor-substitute.md:19,27`）、PR #442（`docs/worklogs/2026-11-18-conductor-substitute.md:19`）。解消の仕方自体は実装者が都度正しく組み立てていた（`docs/worklogs/issue-385.md` の実装者メモ: 両側残しで解消し、宣言だけの取り込みは未使用になるため落とした理由を記録）が、手順化されていないためモデルの能力に依存している | 衝突解消の規律（意図の裏取り・両側残し優先・解消後の同一フロー）がモデルの推論ではなく手順に固定される |
| `hk.pkl` の exclude に `.eval-workspace/**` を追加 | `mise run verify` の hk-check が gitignore 済みの `.eval-workspace/` 内の成果物（benchmark.md の末尾改行・PR 本文下書きの凍結 ID の引用）を検査・自動修正対象にした（実行ログ。#509 で導入した作業領域） | コミットしない実行成果物が検査で落とされず、自動修正で書き換えられない |
| `--ours` / `--theirs` での解消禁止 | 想定（予防）。Pelway の記録に flag 解消による事故は無い。mattpocock 版の根拠（「解決が構文的に正しくても、誰かが意図的に入れた変更を黙って落としうる」）による | flag 解消による意図の喪失の防止 |

## 想定と実績の区別

- 裏付け済み: 衝突解消の修正依頼の反復（3 回）、実装者が意図の裏取りを自力で行えている実績（issue-385）、周辺フローの失敗（旧 approve のままでの報告 = PR #388 インシデント、PR #509 で対応済み）。
- 想定（一次記録無し）: flag 解消（`--ours`/`--theirs`）による意図の喪失の発生可能性。

## 実装者メモ（round 1）

対象 head: 790a69e6d2c2fe3467313d274a4e03b399a52225（push 後の署名付け替え後の SHA。ローカルの f8c200f から付け替え）

1. 詰まった箇所: ブランチ戦略。PR #509（同時に OPEN、同じ手順 9 の上側を改訂）と本文が重ならないよう、挿入位置を Request changes 節と Approve 節の間（#509 が触っていない領域）に置き、origin/main 基点の独立 PR にした。マージ順に依存しない。
2. 曖昧だった文: 「It refuses to treat a conflict as a text problem.」（mattpocock/skills docs/engineering/resolving-merge-conflicts.md）— 「テキスト問題として扱わない」の具体的な手順が本文に書かれて初めて実行可能になるため、原則を手順の形（一次情報への遡り・flag 禁止）に翻訳した。
3. 仮定と解釈: 「修正指示」には指揮役の CONFLICTING 解消依頼だけでなく、レビュー結果を介さない他の修正指示も含むと解釈し、見出しを「レビュー結果を介さない修正指示」と一般化した。今後その種の指示（QA 点検等）が増えた場合も同じ節で扱う。
4. 足りなかった情報・検証手段: 衝突解消の振る舞いの回帰 eval（凍結環境で merge 中の worktree を再現する必要があるため、`evals/agent-team` の現形式では environment/ に git 状態の写しを置く形になる。未実施）。
5. 改善案: なし（3 行の追加に留めた。よくある失敗表への行追加は、flag 解消の事故の一次記録が無いため見送った）。

使ったスキル: find-skills（ユーザーレベル `~/.agents/skills/find-skills`）、write-pull-request（`.agents/skills/write-pull-request/SKILL.md`）
