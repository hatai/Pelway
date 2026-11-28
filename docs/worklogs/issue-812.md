# Issue #812 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: 画像添付の手順が `docs/agents/` に無いままだと、実装担当と hatai が別々の手順で画像を添付し、PR 本文から参照する場所がぶれる。
  確かめ方: `bash scripts/tests/pr-image-attachment-docs.test.sh` で、手順の各段と App のトークンの制約を書いた文言の存在を確かめる。
- 失敗の筋: `write-issue` だけに注意を書き、実装担当が PR を作るときに読む手順から参照が無いと、スクリーンショットを求める Issue でも添付の約束に辿り着けない。
  確かめ方: 同じテストで、`write-issue` と `implement-issue` の両方から `docs/agents/pr-image-attachments.md` を指す文言の存在を確かめる。
- 失敗の筋: `gh pr comment <N> --attach '<path>#<alt>'` の引用符を落とすと、利用者が `<alt>` の前の `#` を shell のコメントの開始と混ぜて写し、添付が失敗する。
  確かめ方: 同じテストで、引用符を含むコマンド例の文言の存在を確かめる。

## 失敗の筋（round 1 の最終）

- 失敗の筋: 手順書だけを追加し、`write-issue` や `implement-issue` から参照が無いと、スクリーンショットを求める Issue を書くときと実装するときの両方で手順に辿り着けない
  確かめ方: `.agents/skills/write-issue/SKILL.md` と `.agents/skills/implement-issue/SKILL.md` の引用箇所を読み、`bash scripts/tests/pr-image-attachment-docs.test.sh` で必須文言の存在を確かめる
- 失敗の筋: `gh pr comment <N> --attach '<path>#<alt>'` の引用符を落とすと、利用者が `<alt>` の前の `#` を shell のコメントの開始と混ぜて写し、添付が失敗する
  確かめ方: `docs/agents/pr-image-attachments.md` のコマンド例を読み、`bash scripts/tests/pr-image-attachment-docs.test.sh` で引用符を含むコマンド例の存在を確かめる
- 失敗の筋: 実装担当が人間名義の道具を使えるように読める文面だと、hatai が文面を選んでいない投稿が hatai 名義で出る
  確かめ方: `docs/agents/pr-image-attachments.md` の前提と手順 2 を読み、実装担当はコマンドと文面を示すだけで hatai が実行することを確かめる
- 失敗の筋: コメント URL を確かめる前に PR 本文へ画像の添付済みと書くと、画像の証拠が無い PR が通ったように見える
  確かめ方: `docs/agents/pr-image-attachments.md` の手順 4〜5 を読み、URL を確かめるまで添付済みと書かないことを確かめる

## 実装者メモ（round 1）

対象 head: 795bbb473b97e25221b92540fd447b03f5b41a7a

1. 詰まった箇所: 手順 6: 1 回目の `mise run verify` は `.agents/skills/verify-pelway/verify-run.test.sh` で失敗した。この sandbox の `/tmp` の所有者が uid 65534 で、`dir_chain_safe` の所有者検査に合わないためで、自分の変更を含まない最小の呼び出しでも同じ失敗を再現した。`TMPDIR=/dev/shm/p812` でやり直し、exit=0・`script-test: 69 tests ok`・`Finished in 451.39s` で通った。2 回目のログには、先に終わりきっていなかった 1 回目の出力が混ざり、末尾に古い失敗行も残った。
2. 曖昧だった文: 「hatai に hatai 名義のコメントでの添付を確かめる」（Issue #812「受け入れ条件」）
3. 仮定と解釈: 実装担当は人間名義のコマンドを実行せず、hatai が実行するコマンドとコメント文を示して確認する意味と解釈した。実装担当が人間名義の道具を使う解釈は選ばなかった。
4. 足りなかった情報・検証手段: `gh pr comment --attach` が GitHub App のトークンで失敗する公開の仕様は見つからず、Issue #812 の背景と PR #744 の hatai 名義のコメントを根拠にした。
5. 改善案: `.agents/skills/command-team/SKILL.md` の「レビュー担当と effort」に「既定は `medium`」と書く（この値は `agent-team.toml` の `effort.raise` と Issue #670 のメモの解釈にそろえる）。

使ったスキル: implement-issue、write-pull-request、writing-for-agents、stop-ai-slop-jp、ja-text-communication、diagnosing-bugs

## 失敗の筋の見直し（round 2）

- 失敗の筋: hatai に確認させたコメント文がコマンドに渡されず、画像だけのコメントが投稿される。
  確かめ方: `docs/agents/pr-image-attachments.md` のコマンド例に `--body-file` があることを `bash scripts/tests/pr-image-attachment-docs.test.sh` で確かめる。
- 失敗の筋: `<path>`・`<alt>`・`<body-path>` に単一引用符が入ると、hatai に示したコマンドの引用符が閉じ、残りが shell で実行される。
  確かめ方: 同じテストで、実装担当が改行と単一引用符を含まない値を選び、hatai がそれらを拒否条件にする文言の存在を確かめる。
- 失敗の筋: `素の gh` を例外と書くと AGENTS.md の例外一覧と食い違い、実装担当が素の `gh` を使ってよいと読む。
  確かめ方: 手順 3 が hatai 本人の実行をエージェントの規則の対象外と書くことを確認する。
- 失敗の筋: App 全般の仕様のように書くと、観測だけの根拠を公開仕様と誤認する。
  確かめ方: 前提が `pelway-dev` のトークンでの観測に限定し、公開仕様の確認ではないと書くことを確認する。

## 失敗の筋（round 2 の最終）

- 失敗の筋: 手順書だけを追加し、`write-issue` や `implement-issue` から参照が無いと、スクリーンショットを求める Issue を書くときと実装するときの両方で手順に辿り着けない。
  確かめ方: `.agents/skills/write-issue/SKILL.md` と `.agents/skills/implement-issue/SKILL.md` の引用箇所を読み、`bash scripts/tests/pr-image-attachment-docs.test.sh` で必須文言の存在を確かめる。
- 失敗の筋: hatai に確認させたコメント文がコマンドに渡されず、画像だけのコメントが投稿される。
  確かめ方: `docs/agents/pr-image-attachments.md` のコマンド例に `--body-file` があることを `bash scripts/tests/pr-image-attachment-docs.test.sh` で確かめる。
- 失敗の筋: `<path>`・`<alt>`・`<body-path>` に単一引用符が入ると、hatai に示したコマンドの引用符が閉じ、残りが shell で実行される。
  確かめ方: 同じテストで、実装担当が改行と単一引用符を含まない値を選び、hatai がそれらを拒否条件にする文言の存在を確かめる。
- 失敗の筋: 実装担当が人間名義の道具を使えるように読める文面だと、hatai が文面を選んでいない投稿が hatai 名義で出る。
  確かめ方: `docs/agents/pr-image-attachments.md` の前提と手順 2 を読み、実装担当はコマンドと文面を示すだけで hatai が実行することを確かめる。
- 失敗の筋: `素の gh` を例外と書くと AGENTS.md の例外一覧と食い違い、実装担当が素の `gh` を使ってよいと読む。
  確かめ方: 手順 3 が hatai 本人の実行をエージェントの規則の対象外と書くことを確認する。
- 失敗の筋: App 全般の仕様のように書くと、観測だけの根拠を公開仕様と誤認する。
  確かめ方: 前提が `pelway-dev` のトークンでの観測に限定し、公開仕様の確認ではないと書くことを確認する。
- 失敗の筋: コメント URL を確かめる前に PR 本文へ画像の添付済みと書くと、画像の証拠が無い PR が通ったように見える。
  確かめ方: `docs/agents/pr-image-attachments.md` の手順 4〜5 を読み、URL を確かめるまで添付済みと書かないことを確かめる。

## 実装者メモ（round 2）

対象 head: 987d6c407df5da8cc79e91f5f0299a0346735182

1. 詰まった箇所: 手順 9: devin のレビュー子タスクは `CHANGES_REQUESTED` を投稿したが、完了報告に `pr:` / `verdict:` / `must-fix:` が無かった。投稿は現在の head と委託記録に一致したため、報告を確かめられない実行の投稿と解釈し、`CHANGES_REQUESTED` の must-fix を結び付かない投稿の決まりで対応した。検証は `TMPDIR=/dev/shm/p812-r2 mise run verify` で exit=0、`script-test: 69 tests ok`、`Finished in 283.45s`。
2. 曖昧だった文: 「path と alt に `'` を含めないことを実装担当の確認と hatai の拒否条件に書くか、shell の quoting を明示する」（devin のレビュー）。
3. 仮定と解釈: 手作業で写すコマンドでは shell の quoting の説明より、改行と単一引用符を含まない値に限定する方が確かめやすいと解釈した。`pelway-dev` のトークンでの観測を GitHub App 全般の公開仕様と区別し、「この手順では扱う」形にした。素の `gh` は例外を増やすのではなく、hatai 本人の実行はエージェントの規則の対象外と書く形にした。
4. 足りなかった情報・検証手段: Devin 子タスクの完了報告から最後の判定行が欠ける原因と、`verdict: failed` のラウンドサマリーを履歴の `<R>` に数えるかの正本が無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 に「投稿済みだが完了報告の判定行が無い実行の扱い」と、「`verdict: failed` のラウンドサマリーが履歴の `<R>` に与える影響」を足す。

使ったスキル: implement-issue、write-pull-request、ja-text-communication、stop-ai-slop-jp
