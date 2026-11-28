# Issue #794 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: 対象パスの glob や inline unit test の補足が狭すぎると、DB・試験・クライアント UI を変更した PR でも対応する観点ファイルが選ばれず、領域固有の欠陥を見落とす。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で対象パスと inline test 補足の文言を確かめ、PR 本文に引用した `review-pr` の選択手順をレビュー担当が読む。
- 失敗の筋: 「報告しないもの」と中央の must-fix の優先順が曖昧だと、報告してはいけない範囲外の指摘を nit に載せたり、逆に観点ファイルの表現を根拠に中央の must-fix を下げたりする。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で中央ルール優先と fail-closed の文言を確かめ、PR 本文に引用した規範をレビュー担当が読む。
- 失敗の筋: 観点ファイルを PR の版から読むと、その PR で規則を弱めた変更が自分自身のレビュー規則を差し替え、検査を迂回できる。
  確かめ方: `.agents/skills/review-pr/SKILL.md` の「領域別のレビュー観点ファイル」で main の版を読む手順を確認し、presence check で front matter を先に読む文言を確かめる。

## 失敗の筋の見直し（round 2）

- 失敗の筋: root の `Cargo.toml`・`Cargo.lock`、`mise.toml`、`.github/workflows/`、`deny.toml` だけの変更でも、差分中の文字列が偶然 inline test の条件に見えると試験領域が発動し、レビュー範囲が不必要に広がる。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で root-only 変更の除外と Rust ファイル限定の文言を確かめる。
- 失敗の筋: 再レビューで観点ファイル選択だけが main の merge commit や `diverged` の例外を無視すると、レビュー本体と違う差分に対して観点を選び、適用漏れや余計な適用が起きる。
  確かめ方: `.agents/skills/review-pr/SKILL.md` で観点ファイル選択が「2 回目以降の round」の差分を参照することを確認する。

## 失敗の筋（round 2 の最終）

- 失敗の筋: 対象パスの glob や inline unit test の補足が狭すぎると、DB・試験・クライアント UI を変更した PR でも対応する観点ファイルが選ばれず、領域固有の欠陥を見落とす。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で対象パスと inline test 補足の文言を確かめ、PR 本文に引用した `review-pr` の選択手順をレビュー担当が読む。
- 失敗の筋: 対象パスの glob や inline unit test の補足が広すぎると、root-only 変更や Rust ではない文書の変更にまで試験領域が発動し、レビュー範囲が不必要に広がる。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で root-only 変更の除外と Rust ファイル限定の文言を確かめる。
- 失敗の筋: 「報告しないもの」と中央の must-fix の優先順が曖昧だと、報告してはいけない範囲外の指摘を nit に載せたり、逆に観点ファイルの表現を根拠に中央の must-fix を下げたりする。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で中央ルール優先と fail-closed の文言を確かめ、PR 本文に引用した規範をレビュー担当が読む。
- 失敗の筋: 観点ファイルを PR の版から読むと、その PR で規則を弱めた変更が自分自身のレビュー規則を差し替え、検査を迂回できる。
  確かめ方: `.agents/skills/review-pr/SKILL.md` の「領域別のレビュー観点ファイル」で main の版を読む手順を確認し、presence check で front matter を先に読む文言を確かめる。
- 失敗の筋: 再レビューで観点ファイル選択だけが main の merge commit や `diverged` の例外を無視すると、レビュー本体と違う差分に対して観点を選び、適用漏れや余計な適用が起きる。
  確かめ方: `.agents/skills/review-pr/SKILL.md` で観点ファイル選択が「2 回目以降の round」の差分を参照することを確認する。
- 失敗の筋: 複数領域にまたがる指摘の must-fix・nit の優先順が未定義だと、同じ指摘の重さが読み手やレビューごとに揺れ、領域固有の欠陥を nit に下げたり逆に過剰な must-fix にしたりする。
  確かめ方: `bash scripts/tests/review-perspective-docs.test.sh` で「明示された must-fix を最優先」の文言を確かめ、PR 本文に引用した衝突解決の規則をレビュー担当が読む。

## 実装者メモ（round 1）

対象 head: 507149f0b5f5622766890dbe4372410a036e7300

1. 詰まった箇所: 手順 6: `scripts/tests/review-perspective-docs.test.sh` の Rust 属性の wording pin が shellcheck SC2016 に当たったため、既存の方針どおり行ごとの disable で回避した。`mise run verify` では `aws-lc-sys` の並行ビルドで object file が欠ける失敗と、 `/tmp` の所有者を safe hierarchy と認めない verify-pelway の失敗が出たため、`cargo clean -p aws-lc-sys` と実行ごとの `TMPDIR=/dev/shm/...` で切り分けた。
2. 曖昧だった文: 「領域（少なくとも DB の移行と契約・試験・クライアントの UI）ごとに、対象のパス・報告するもの・報告しないもの・既定の重さを書いた観点のファイルが `docs/agents/` にある → ファイルを読む」（Issue #794「受け入れ条件」）
3. 仮定と解釈: 「契約」は backend の `/crates/pelway-contract/**` だけとし、Electron IPC の `client/packages/contracts/` は含めないと解釈した。glob の正本は各観点ファイルの front matter とし、Rust の inline unit test はパスではなく diff 内容で捕捉すると解釈した。
4. 足りなかった情報・検証手段: glob と変更パスの一致を機械的に確かめる helper が無く、レビュー担当が手で照合する必要がある。
5. 改善案: `docs/agents/review/README.md` の「領域の追加」に「glob の意味、複数一致、inline unit test の補足を書く」を足す。

使ったスキル: implement-issue, write-pull-request, improve-skills, writing-for-agents, `.agents/skills/tdd`（seam: `scripts/tests/review-perspective-docs.test.sh` の presence check）, `.agents/skills/diagnosing-bugs`

### improve-skills 手順 7 の引き渡し記録

- 候補の出どころ: `docs/worklogs/issue-794.md` round 1「改善案」
- 対象の改善点: `.agents/skills/review-pr` に領域別の「報告するもの」「報告しないもの」「既定の重さ」を読む手順が無かった（手順の欠落）
- 期待する変化: レビュー担当が領域固有の must-fix を見落とす手戻りと、範囲外・証拠の無い nit を繰り返す手戻りが減る

## 実装者メモ（round 2）

対象 head: 6b93e81bf31ba7585e4a331d50e8773da988d743

1. 詰まった箇所: 手順 9: サブエージェントへの設計入力が conflict 時の優先順と既定値変更時の理由記録を欠いていたため、最初の Spec レビューに誤報が出た。完全な設計を渡して再実行し、誤報と実指摘を分けた。
2. 曖昧だった文: 「2 回目以降の round では、前回レビューした sha（`Reviewed:` の行）から今の head までの差分と、前回の must-fix が直ったかを見る。」（`.agents/skills/review-pr/SKILL.md`「2 回目以降の round」）
3. 仮定と解釈: 観点ファイル選択の差分は、この既存の round 範囲と同じであり、main の取り込みを除く例外と `diverged` 時の全体レビューも引き継ぐと解釈した。「Rust の inline unit test」は `.rs` ファイルの差分に限定すると解釈した。
4. 足りなかった情報・検証手段: glob の一致を機械的に確かめる helper が無く、root-only 変更や `.rs` 限定の回帰は wording pin でしか確かめられない。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の「2 回目以降の round」を「レビュー対象の差分」という見出しに分け、初回・再レビュー・観点ファイル選択が同じ正本を参照できるようにする。

使ったスキル: implement-issue, write-pull-request, code-review, `.agents/skills/tdd`（seam: `scripts/tests/review-perspective-docs.test.sh` の presence check）

## 実装者メモ（round 3）

対象 head: defc31c3c375c11ed9670883f856f914483db1fe

1. 詰まった箇所: 手順 6: main の取り込みで `review-pr` の must-fix 表と検査欄が衝突した。main の「失敗の筋」要件とこの PR の観点ファイル要件を両方残した。また、merge commit を含む push は `agent-gh-resign` が署名付け替えできず、後続コミットも署名なしのままになった。
2. 曖昧だった文: 「Draft PR を作る前に `## 失敗の筋（最終）` を書く。」（`.agents/skills/implement-issue/SKILL.md`「7. Draft PR を作る」）
3. 仮定と解釈: この PR は新しい要件が main に入る前に作られているが、レビューは main の規則で行われるため、失敗の筋を遡って記録する。`review-pr` は記録時刻の証明を求めないため、実装前・round 2・最終の内容を現在の差分に合わせて残す解釈を選んだ。
4. 足りなかった情報・検証手段: PR 本文が新しい template の必須欄を満たすかを push 前に機械的に確かめる helper が無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「修正依頼」に「main の取り込みで PR template か review-pr が変わったときは、新しい必須欄を PR 本文と作業ログに反映してからレビューに戻る」を足す。

使ったスキル: implement-issue, write-pull-request, fix-merge-conflicts
