# PR への画像添付

AI 開発チームが PR にスクリーンショットなどの画像を添えるときの手順。

## 前提

- GitHub App のトークンでは画像を添付できない。`gh pr comment` や `gh pr edit` の `--attach` は `attaching files requires write access to the repository` で失敗する。
- 人間のアカウントで投稿する道具は、hatai が文面か選択肢を明示して選んだときだけ使う。実装担当は hatai の名義でコマンドを実行しない。

## 手順

1. 実装担当が画像を用意する。画像ファイルは hatai から見えるパスに置き、PR 番号・ファイルのパス・alt テキスト・コメントの文面を hatai に示す。PR がまだ無ければ、先に Draft PR を作る。
2. hatai に hatai 名義のコメントでの添付を確かめる。実装担当は、hatai が実行するコマンドとコメント文をそのまま示し、hatai の確認なしに人間名義の道具を使わない。
3. hatai は次の形で画像を添付する。`<N>` は PR 番号、`<path>` は画像ファイルのパス、`<alt>` は画像の代替テキスト。このコマンドは hatai の名義で実行するため、例外的に素の `gh` を使う。

   ```bash
   gh pr comment <N> --attach '<path>#<alt>'
   ```

4. 実装担当は、hatai のコメントの URL を GitHub から読み、PR 本文からそのコメントを指す。PR 本文の編集は `scripts/agent-gh dev pr edit` で行う。
5. 受け入れ条件で画像を求めるときは、実装担当は PR 本文の「確認方法/検証」に、コメントの URL と画像に写っている対象を書く。コメントの URL を確かめる前に、画像の添付が済んだものとして書かない。
