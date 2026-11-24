# Issue #642 の作業ログ

## 実装者メモ（round 1）

対象 head: f6fd802ce87e39eea10e6ca3b37fef4004caffc8

1. 詰まった箇所: 前任スレッドの worktree（`agent-642-pre-push-pr-s-88fb15158d2ceef0`）がこの sandbox では読み取り専用で merge も checkout もできなかった。自分の worktree を `git switch --ignore-other-worktrees` で同じブランチに切り替え、`origin/main` を merge して #641 の `push_targets` を取り込んだ。fake gh の `issue view` は `${VAR:-{"comments":[]}}` の既定値が `}` をはみ出して余分な `}` を出し jq の parse error になった（`:-` の中の `}` を引用符で囲んで修正）。
2. 曖昧だった文: 「例外が認められていなければ push を止める」（Issue #642 受け入れ条件）— 例外の判定手順（PR のラベルと Issue の marker コメントの組）は本文の別節に書かれており、両方を読み合わせる必要があった。
3. 仮定と解釈: 数える対象は refspec の src（`HEAD:dst` なら HEAD、`src:dst` なら src）とした。`git push origin HEAD` の形で別ブランチを push する場合、HEAD の差分ではなく push される sha の差分を測るのが Issue の意図（「push する差分」）と解釈し、`push_targets` に `push_srcs` を並列配列として足した。base は `branch.<ローカルブランチ>.gh-merge-base`（`src` からローカルブランチ名を引く）→ `origin/main` とし、未決事項にあった「PR の base 優先」は採用しなかった（hatai が受け入れ条件を書き換えずに ready-for-agent を付けたため、本文どおり）。
4. 足りなかった情報・検証手段: `review git push` が gate の対象外であることは本文に明示されていたが、gate の中身は `role == dev` のときだけ呼ぶ形で実現し、review 役では `push_targets` の照会（`git config` 系）だけが走ることを既存試験で確かめた。
5. 改善案: `scripts/agent-gh` の push 処理に条件分岐を足すときは、`push_targets` が埋める配列（dst だけでなく src も）を使い回して引数解析を二重に書かないこと。

使ったスキル: implement-issue

## 実装者メモ（round 2・codex r1 must-fix 対応）

対象 head: 未 push（このメモを含むコミット）

1. 設計の決め手: must-fix の 1（refspec 省略）・2（タグ名前空間）・3（測定 sha と送信 sha の一致）を 1 つの機構で解くため、`git push --dry-run --porcelain` で Git が実際に更新する ref を解決し、porcelain の `<旗>\t<src>:<dst>` 行で `refs/heads/` への ` `・`+`・`*` 更新だけを検査対象にした。`=`（最新のまま）・`!`（拒否）・`-`（削除）は実際に送られないため対象外。
2. sha の pin（must-fix 3）: 解決した src を `rev-parse` で sha に固定してから測定し、検査を通ったら実 push の引数を `<sha>:<dst>` に組み替える（`pin_push_argv`）。測定と push の間に別 worktree がブランチを動かしても、測った sha が送られる。pin できない形（`--mirror`・`--prune`・glob refspec）は exit 6 で止める（fail-closed）。`--all`/`--branches` は旗を外して解決した refspec に展開する。
3. refspec 省略・リモートだけの push（must-fix 1）: dry-run が upstream・`push.default`・引数をすべて解決するため、その出力を検査するだけで網羅できる。dry-run 自体が失敗した場合は検査対象なしとして実 push に回す（upstream 未設定・接続失敗では実 push も同じ理由で失敗し、ブランチは更新されない）。
4. 例外照合のリポジトリ識別（must-fix 4）: `gh pr list --repo <owner/repo>` に headRepository.nameWithOwner + headRefName を取らせ、head のリポジトリとブランチの両方が push 先と一致する PR だけを例外対象にした（同名ブランチを持つフォークの承認を使わない）。closingIssuesReferences は `url`（無ければ `repository.nameWithOwner` + `number` の `owner/repo#n` 形）で issue view するため、別リポジトリの同番号 Issue を取り違えない。push 先の owner/repo は `remote.<名>.pushurl`/`.url` の設定値（insteadOf 書き換え前）から取り、解けなければ fail-closed。
5. 測定不能の扱い（must-fix 5）: `git diff --numstat` はプロセス置換の終了コードが親に伝わらないため、いったん mktemp のファイルに出して正常終了を確かめてから読む。失敗（共通祖先が無い等）は終了コード 3 を返し、ゲートは exit 6 で止める（0 行で通さない）。
6. 試験の stderr 分離（must-fix 6）: `push_gate`/`push_gate_skip` は stdout=OUT・stderr=ERR に分け、行数・base・例外ラベル・hatai・目印マーカーは ERR を照合する。
7. 試験でハマった点: 実リポジトリ試験の origin は `https://github.com/hatai/Pelway.git` にして `url.<bare>.insteadOf` でローカル bare に向ける。`git@github.com:` 形は agent-gh が `-c url.https://github.com/.insteadOf=git@github.com:` で https に書き換えるため credential 待ちでハングした。insteadOf は最長一致でローカルのルールが勝つ。もう 1 つ、通った push が bare に ref を残すと後の試験で同じ sha が `=`（最新のまま）になってゲートの対象外になるため、通した試験の後は `clear_remote` で消す。
8. 曖昧だった文: 「push する差分の測定」を、dry-run が送ると確定した更新の sha に限定した。`=`（remote に既にある sha）は「何も送らない」ため測定しない — remote に既にその差分があるなら gate の意味が無いため。拒否される更新（`!`）も送られないので対象外。
9. 改善案: `push_targets` は引数の形の記録（push_raw/pos_idx/expand_idx）と検査の対象外フラグ（push_no_gate/push_no_pin）をまとめて返すため、push 関連の引数解析はここに集約して増やすこと。

使ったスキル: implement-issue
