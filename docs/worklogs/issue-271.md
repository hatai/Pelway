# Issue #271 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 当初は「push 後に `POST /git/commits` で同内容コミットを作り直す」方式で実装したが、実地確認で `verification.reason=unsigned` と判明。REST Git データ API は署名しないため、署名が得られる `createCommitOnBranch` 方式に作り替えた。`git log --format` の末尾区切り NUL 欠けで `meta[5]: unbound variable` にもなり、`%x00` 追加で解消。
2. 曖昧だった文: 「その名義の認証で git を実行する」（`scripts/agent-gh` の usage 3 行目）——「認証」にコミット署名が含まれるか書かれていない。この機に `commit.gpgsign=false` を明示して署名経路を API 側に一本化した。
3. 仮定と解釈: `createCommitOnBranch` の `fileChanges` では新規ファイルが 100644 固定・mode 変更・symlink・submodule・マージ・空コミットを表現できないため、範囲内に 1 件でも含まれる push は警告して署名なしのままにすると解釈（警告だけで push を壊さない方を選択。拒否しても API 側で直せないため）。既存ファイルの内容変更は mode を保つことを実機プローブ（probe/sign-test ブランチ）で確かめた。push 対象は upstream（無ければ `origin/<現在ブランチ>`）の追跡 ref と仮定。
4. 足りなかった情報・検証手段: REST `git/commits` が署名しないこと・`createCommitOnBranch` の mode 維持・author/committer がサーバ側決定であることは、実リポジトリへのプローブでしか確かめられず、issue 本文の「採用する方式」（REST 方式）が前提から崩れた。
5. 改善案: Issue #271 の「採用する方式」が REST 方式のまま残っているため、`createCommitOnBranch` 方式に本文を書き直すか、差し戻して新規起票にするかを指揮役に仰ぐ（スキル `write-issue` 手順 7）。

## 実装者メモ（round 1 補足: 本 PR 自体の署名について）

`scripts/agent-gh-resign` を新規追加するこの PR のコミットは、新規実行ファイルを含むと表現不能で署名できない。`deny-warn-extract.sh`（100644・bash 呼び出し）の先例に倣い、resign スクリプトを 100644 にして `agent-gh` から `bash` で呼ぶ形に変え、この PR のコミット自体が署名付きになるようにした。
