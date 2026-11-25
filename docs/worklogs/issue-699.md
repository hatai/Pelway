# Issue #699 の作業ログ

## 実装者メモ（round 1）

対象 head: 62392f6881db2b7f0a7bbb0f70518b72b711acc8

1. 詰まった箇所: `scripts/tests/conductor-watch-patrol-state.test.sh` の試験 1: 「symlink を読まない」を `neq "$(since61)" "$SEEDED_SINCE"` で書いたら、記録が symlink のまま残る main でも `since61` が `not-regular` を返して通った（red にならない）。通常ファイルとして書き戻され、かつ時刻が引き継がれていないことを見る `fresh` に書き直した。
2. 曖昧だった文: 「`scripts/conductor-watch.sh` の patrol-state.json を読む箇所すべてに #680 の `read_ps` と同等の検査（O_NOFOLLOW・O_NONBLOCK・通常ファイル限定・サイズ上限）が適用されている」（Issue #699 の「受け入れ条件」）。書き戻し（背景の `:651`）と旧置き場からの移行の `cp` が「読む箇所」に入るかが書かれていない。
3. 仮定と解釈: 背景の `:651` は書き戻しの `open(ps_path, "w")` と仮定した（#680 のマージ直後の main `0c6a50d9` の 651 行目で確かめた）。FIFO の試験は読み取りだけを直しても書き戻しの open で止まるため、書き戻しも範囲に含め、一時ファイルからの置き換えにした。移行の `cp` も旧置き場を読む箇所として含めた。読めない記録は新しい記録で置き換える（既存の「JSON として読めない → 空」と同じ扱い）解釈を選び、書き戻しをやめる解釈は選ばなかった（`conductor-watch-pending-sends.test.sh` 試験 12 の「symlink でも書き戻しは止まらない」と食い違うため）。受信箱にある所有者の検査は、受け入れ条件の 4 つに無いので足さなかった。JSON として読めても辞書でない記録（`[]`）は、指揮役の依頼文の「読めない行を飛ばす」と同じ種類の耐性と読み、空の記録として扱うことにした（main では巡回状態の更新が毎回例外で止まる。試験 11）。
4. 足りなかった情報・検証手段: 移行の `O_EXCL`（`lexists` で確かめた後に置かれた symlink の先に書かない）は競合の窓を試験で作れず、`O_EXCL` を外した変異が生き残った。試験 9（旧置き場の FIFO）と試験 10（一時ファイルの後始末）は main でも通るため、`O_NONBLOCK` と `unlink` をそれぞれ外した変異で red になることを確かめた。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方（このスキルの核）」に「レビューの範囲外指摘から起票するときは、指摘が挙げた行ごとに、それが読み取りか書き込みかを書き、その行を確かめる受け入れ条件を 1 つずつ置く」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 9582275ab05ed16bf20f5ca6b6d5f405dd915a39

1. 詰まった箇所: `scripts/tests/conductor-watch-ensure.test.sh` の fallback・systemd: 移行の python を置き場と旧置き場が同じときにも走らせたら、名前空間の中で `python3`（mise の shim）が「not a valid shim」で動かず、移行の失敗として見張りが終了した。前の版と同じく移行の python は置き場が旧置き場と異なるときだけ走らせ、親ディレクトリの作成は巡回の `open_ps_dir` に任せた。
2. 曖昧だった文: 「既存のボード更新キュー（737–762 行目）と同様に、信頼できる基点から親ディレクトリを検証して開き」（PR #705 の codex r1 レビュー、must-fix 3）。ボード更新キューはキューのディレクトリ自体を O_NOFOLLOW で開くだけで、その親（/tmp）は検証していないため、「信頼できる基点」がどこかが決まらない。
3. 仮定と解釈: 信頼できる基点を記録の親ディレクトリの 1 つ上（既定は git が解決した共有 git dir）と置いた。sandbox は共有 git dir の中には書けるが、`.git` 自体の名前は置き換えられないという前提。「許可ビットを広げない」は、元の記録の許可ビットを写す解釈ではなく、0600 で作る解釈にした（読み手の conductor-watch・pelway-agent-watch・conductor-rebuild・指揮役はどれも同じユーザーで動く。実機の 0644 の記録は 0600 に狭まる）。書き戻さない理由の行は、毎巡回ではなく thread-list・pending-sends と同じく HEARTBEAT_SECS ごとに出し直す形にした。受信箱 `conductor-inbox.jsonl` の読み取りは同じ親ディレクトリの下だが、Issue #699 の範囲（patrol-state.json）の外として親ディレクトリ基準にしなかった。
4. 足りなかった情報・検証手段: 親ディレクトリを確かめてから開くまでの差し替え（競合の窓）は試験で作れず、symlink に置き換えた状態の試験 14 だけで確かめた。移行の lstat による種類の判別は、ソケットでは ENXIO の捕捉でも同じ結果になるため、lstat を外した変異では試験 12 が red にならない（lstat はデバイスを開かないための判別）。
5. 改善案: `scripts/tests/conductor-watch-ensure.test.sh` の冒頭のコメントに「名前空間の中では mise の shim の python3 が動かないため、見張りの起動時に python3 の成功を前提にする処理を足すと、この試験で見張りが終了する」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 3）

対象 head: 487d502a6ac566f28196f30b50bd99aeee0107ce

1. 詰まった箇所: `scripts/tests/conductor-watch-patrol-state.test.sh` の試験 16: 整形で膨らむだけの記録では、修正前の `json.dumps` も手元のメモリでは整形を終えて `reason=too-large` を出し、red にならない。見張りを `ulimit -v 1048576`（1 GiB）の下で動かし、300 段の入れ子で 1 GiB を超える記録を使って、修正前は `MemoryError` の Traceback が出て理由の行が出ないことを `scripts/red-check` で確かめた。
2. 曖昧だった文: 「JSON を少しずつシリアライズして UTF-8 の累計バイト数を数え、上限を超えた時点で処理を打ち切って既存の記録を保持してください」（PR #705 の codex r2 レビュー、must-fix）。整形した断片を一時ファイルへ直接書くのか、手元に溜めるのかが書かれていない。
3. 仮定と解釈: 断片は上限（4 MiB）まで手元に溜め、超えたら一時ファイルを作る前に `too-large` を返す解釈を選んだ（一時ファイルの作成と後始末が要らず、溜める量は上限と断片 1 つまでに収まる）。断片 1 つの大きさは入力の文字列 1 つの escape 後の大きさで決まり、読み取り上限の数倍に収まると置いた。`JSONEncoder(...).iterencode` は `indent` を指定すると `json.dumps` と同じ Python 側の整形を通るため、出力は変わらない前提にした。
4. 足りなかった情報・検証手段: 試験 16 の 1 GiB の仮想メモリの制限は CI の runner でも効く前提で、CI での `ulimit -v` の可否は PR のチェックの結果で確かめる。整形中の実際の最大メモリ量は測っていない。
5. 改善案: `.agents/skills/review-pr/SKILL.md` の must-fix の書き方に「メモリなど資源の上限の指摘には、red にする条件（制限の値や入力の形）を添える」を足す。

使ったスキル: implement-issue
