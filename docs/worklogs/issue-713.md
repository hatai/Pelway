# Issue #713 の作業ログ

## 実装者メモ（round 1）

対象 head: 51ade39d8be1d6866f6980513fbec10d2caae3c3
PR: #718（この Issue の実装 PR。レビューで確かめる実装の head はこの「対象 head」。メモのコミットはその後ろに乗るため PR の headRefOid と 1 つずれる）

1. 詰まった箇所: scripts/tests/agent-team-config.test.sh: 引数で開発者の表を差し替える補助関数を unquoted heredoc で書いたら、既定値の `"` が quote removal で消えて `normal = [devin, opencode]` の壊れた TOML を生成していた。関数をやめて各試験で表を全部書く形に直した
2. 曖昧だった文: 「上限が 1 未満または整数でないとき、検証が終了コード 0 以外で止まり、違反したキーを伝える」（Issue #713「受け入れ条件」）
3. 仮定と解釈: 「違反したキー」をドット区切りのキーパス（例 `developers.dev.slots.opencode`）と解釈した（キー名だけでも伝わるが、開発者が増えたとき一意に決まるパスの方が誤りを指せる）。レビューの同時数の目安も上限と同じく 1 以上の整数で検証した（「目安」でも 0・非整数の運用は無いため）。`use` の実装・レビューの役割を持つ担当には上限のキーを必須にした（書かなければ無制限と読めるため。「使える担当に書かないことで表す」の決まりは利用可否だけに掛かると読んだ）。codex の effort の段は文書に固定の一覧が無かったため、作業ログで実際に使われた値（low・medium・high・xhigh）から取った。「到達点はマージまでとする」は、実装者メモにこの Issue の PR の番号を書くことと解釈した
4. 足りなかった情報・検証手段: `codex_review` の `reasoningEffort` と `claude_review` の `effort` の有効値の一覧が文書に無い（issue-352 の改善案「レビュー担当の Claude に effort の有効値を明記する」は未適用のまま）
5. 改善案: `docs/dev-environment.md` の「レビュー担当の Claude」に「`effort` の有効値は low・medium・high・xhigh・max」の 1 行を足す（「レビュー担当の Codex」にも同様に `reasoningEffort` の有効値を書く）

使ったスキル: .agents/skills/implement-issue、.agents/skills/diagnosing-bugs（`verify-pelway/verify-run.test.sh` の失敗の切り分けに Phase 2。`scripts/red-check origin/main` で同じ失敗を再現し、環境起因と確かめた）

## 実装者メモ（round 2）

対象 head: 722e52061ce5d4e19d14eb8e5b51603276b83fce
PR: #718

レビュー指摘（run 7160627617 codex request-changes、run 7160628307 claude request-changes、head 1cd33ee2）への対応:

1. 詰まった箇所: なし
2. must-fix の対応:
   - `slots` が `validate` と検証を共有していなかった → `validate_doc` に括り出し、`do_slots` も同じ検証を通してから出す形にした（違反時は標準出力が空のまま非 0）。試験 15（未登録の担当・枠の欠落・途中の上限値の誤りで非 0 かつ標準出力が空）を足した
   - 試験 3 が codex のインスタンス名しか固定していなかった → 4 担当すべてで、役割ごとのインスタンス名の表全体とモデルの一覧全体を比較する形にした
   - 試験 8 の両入力が優先順位の検査でも落ちる入力だった → `developers.dev.use: 実装の役割を持つ担当が 1 つも無い`・`developers.dev.use: レビューの役割を持つ担当が 1 つも無い` の診断を各試験で確認するようにした（優先順位の入力は、実装・レビュー担当 0 種の表では正当な優先順位が書けないため残した。診断の確認で `use` の検査そのものを外せなくなる）
   - 実装者メモに PR の番号が無かった → round 1・round 2 の両方に `PR: #718` を書いた
3. nit の対応: claude の `effort.raise` に `xhigh` が無い → docs/worklogs/ で `claude effort=xhigh`・`claude effort=max`・`codex reasoningEffort=max` の委託実績と、claude のレビュー担当が確認したハーネスの列挙（low・medium・high・xhigh・max）から、codex・claude の両方の `raise` に `xhigh`・`max` を足し、値の根拠をコメントに書いた（有効値の一覧が文書に無い件は round 1 メモ 4 項目めのまま）。[文書のずれ]「対象 head が PR の head と違う」は、対象 head は実装のコミットを指す規則（メモのコミットはその後ろに乗る）を round 1 の PR: の行に注記した
4. PR 本文の nit: 「固定値の確認は合成 TOML で行い」は試験 2・3 がリポジトリの agent-team.toml を読む実態と違う → 本文を実際の試験に合わせて直す
