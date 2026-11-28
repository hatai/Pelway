# Issue #814 の作業ログ

## 実装者メモ（round 1）

対象 head: 54f8372417c8b297615f3a48cb8f0e94c0c00364

1. 詰まった箇所: 手順 4: 受け入れ条件の「合成データのテストで 4 件の扱いを確かめる」の主語が曖昧で、見張り（書き出し側）と指揮役（読み取り側）のどちらを試験するかで迷った。読み取り側は LLM エージェントの手順なので決定的に試験できず、`scripts/board-snapshot-read` という検証用の読み取り手順を新設してそこに 4 件の分岐（正常・stale・broken・missing）を持たせる設計にした。
2. 曖昧だった文: 「巡回はボードの Todo を `gh project item-list` の返却順（全件取得、上限で打ち切らない）で読み、`ready-for-agent` 付きを並び順に割り当てる。」（docs/agents/grokbot-runbook.md「2. ボードと巡回」）
3. 仮定と解釈: 「全件取得、上限で打ち切らない」はスナップショットの取得側の要件でもあると解釈し、`item-list --limit 1000` の打ち切りを検知（`totalCount` が件数を超える）したら公開しない fail-closed にした。案 A を選んだ理由: 書き込みキューと同じ中継の型で秘密を指揮役の環境に入れず、案 B と違い読み取りの鮮度が巡回ごとに自立的に更新され、案 C と違いトークンの発行・保管（人間の作業）を要しない。採らなかったもの: 案 B（送り手のルーティンが別途要る）、案 C（secret の sandbox 持ち込み）。
4. 足りなかった情報・検証手段: `gh project item-list --format json` の実際のキー（`labels`・`assignees` が文字列の配列かオブジェクトか）は、人間名義の gh 認証がこの環境に無く実測できなかった。cli/cli の利用例から文字列の配列と読み、オブジェクト形も正規化で受け付ける防御的な書き方にした。
5. 改善案: docs/agents/grokbot-runbook.md の「ボード読み取りスナップショット」に「`labels`・`assignees` の生の形が gh の版で変わったときは、見張りの正規化（names）とこの節の項目の説明を両方直す」を足すとよい。

使ったスキル: implement-issue。輸入スキルはなし（試行の割り当てではない）

push 後の修正: スナップショットの鮮度判定が `now - fetched < interval` だけで、未来の fetched_at を「新しい」と扱ってしまい、reader が broken と読む書き出しを見張りが更新しなくなる問題に気づき、回帰テスト（future fetched_at is refetched。実装なしで FAIL、実装で ok）を足して `0 <=` の条件に直した（54f8372）。

## 実装者メモ（round 2）

対象 head: 912e995e47d06764cca92737fe6d774e48ecfb51

1. 詰まった箇所: codex の must-fix「書き出し元を検証せずに受け入れる」は、エージェント環境とホストが同じ利用者 ID で同じ `/tmp` を共有する前提では、ファイルの所有者や権限では区別できない。所有権の確認は同じ uid のエージェントを弾けないため、結論として「中身を信用しない」設計（鮮度はプロセス内の記憶、承認は GitHub で読み直し）に倒した。仮定: 共有の `/tmp` では真正性をファイルの属性では確かめられず、行動側の検証で埋めるのがこの前提で取れる形。
2. 曖昧だった文: レビューの「全ページを取得し」は gh project item-list にカーソルの引数が無いため厳密には読めなかったが、gh が limit の件数まで内部でページをたどることを手掛かりに、打ち切りを検出したら totalCount を limit に取り直す形で「全件を確認できた取得だけ公開する」を実現した。
3. 仮定と解釈: カードの型・番号が読めない項目を含む応答は、欠けた一覧を完全なものとして渡さないため公開しない（fail-closed）と解釈した（レビュー指摘どおり）。labels・assignees の要素の正規化（文字列か {"name"} か）は項目レベルの融通として残した。
4. 確かめられなかったこと: `gh project item-list` が内部でページをたどることは gh のドキュメントと動作の説明に依る（実機で 1000 件を超えるボードでは確かめていない）。`totalCount` による取り直しはモックで確かめた。
5. 判断したこと: round 1 のレビュー（codex・claude とも CHANGES_REQUESTED）に対し、(a) 鮮度管理を見張りのプロセス内の記憶に移し共有ファイルを一切読まない、(b) 打ち切りは totalCount で取り直し全件確認できた取得だけ公開、(c) カードの型を fail-closed に、(d) reader は有限性と型を検証し例外でも契約 JSON を返す、(e) runbook に真正性の扱い（承認は GitHub で読み直し）とボード依存処理の保留を書いた。

## 実装者メモ（round 3）

対象 head: 1a9214da5f2cfc832d112bd5cf0d68a48363c735

1. 詰まった箇所: codex の must-fix「列の移動は GitHub で読み直せない」と claude の must-fix「所有者の確認が無い」は同じ根（共有の置き場の真正性）の表裏で、claude が示したとおり同じ uid の偽装は main のキューにすでにある前提だった。codex の「ホストだけが更新できる経路」は uid の分離が無い環境では OS の境界として作れないため、所有者チェック（別 uid の拒否）+ 信頼の前提の明文化で両方に答える形にした。
2. 曖昧だった文: 「ホストだけが更新でき、指揮役が読み取れる経路で書き出しを渡すか、これらの操作の直前に信頼できる経路でカードの実状態を確認してください」（codex の round 2 レビュー）——後者の「信頼できる経路」は列については存在しない（指揮役は人間名義の gh を持たず、列は GitHub の Issue API には出ない）。取れるのは前者に相当する防御のうち uid の境界だけで、同じ uid はキューと同じ前提の内側と明記する解釈を選んだ。
3. 仮定と解釈: 同じ種類の must-fix（共有ファイルの真正性・認可）が 2 round 続いたため、種類の全経路に当てる決まりを「共有の置き場（/tmp など複数の主体が書ける場所）を経由する入力は、キュー・スナップショット・巡回状態のどれでも、(a) 所有者が自分の uid であることを確かめ、(b) 中身はデータとしてだけ扱い、(c) 承認に関わる判断は信頼できる経路で読み直すか読み直せないことを明記する」と決めて直した。スナップショットの経路に (a) が欠けていたのが round 2 の指摘で、キューと巡回状態は main の時点で (a) を持つ。仮定: sandbox の中でも見張りの書いたファイルは自分の uid に見える（同一の /tmp の共有で uid の写像が変わらない）。この環境では実機の uid 対応は確かめられない。
4. 足りなかった情報・検証手段: 見張りと指揮役の実際の uid の関係（sandbox の内外で同じ uid に見えるか）はこの環境からは確かめられず、テストでは PELWAY_BOARD_SNAPSHOT_EXPECT_UID で比べる uid を差し替える形にした。4 MiB の境界の実データは重いため、writer 側もテスト用の上限（PELWAY_BOARD_SNAPSHOT_MAX_BYTES）で縮めて境界を確かめた。
5. 改善案: scripts/board-snapshot-read の冒頭の説明に「真正性の前提（所有者が同じ uid であることだけが確かめられること）」を 1 行足すと、読み手が runbook を開かなくても前提を追える。

使ったスキル: implement-issue。輸入スキルはなし

## 実装者メモ（round 4）

対象 head: 102da796bcf473cb2e1669fa75b28c4d6c7f954a

1. 詰まった箇所: main のマージで `.devin/config.json` が sandbox の読み取り専用マウントで更新できず `git merge` が失敗した。コミットは worktree のそのファイルに触れずに作れるため、`git merge-tree --write-tree` → `commit-tree` → `read-tree` でマージコミットだけを作り、書けるファイルだけを checkout で合わせた。worktree の `.devin/config.json` は古いままで、以後のコミットに含めないようファイルを個別に add する。
2. 曖昧だった文: 「UTF-8 以外のロケールでラベル名などに非 ASCII 文字があると、容量の検査を通った後に、reader が解釈できないバイト列で有効な前回分を置き換える場合があります」（codex round 3 レビュー）——再現の確かめで `python3 -I` は `PYTHONUTF8` を無視すること、C ロケールは PEP 538 で UTF-8 に矯正されることが分かり、環境変数だけではこの状態を作れなかった。テストは mock python3 で `-X utf8=0` を差し込み、UTF-8 モードを強制できないホストを模す形にした。
3. 仮定と解釈: must-fix 1 は、見張りの状態ディレクトリが sandbox から読み取り専用で見えることをこの環境で実測してから進めた（`/home` が ro マウント、`~/.local/state` への書き込みは Read-only file system で失敗、ファイルの所有者は自分と同じ uid 1000 に見える）。符号化の指摘は書き込み側だけでなく gh の出力の読み取り側も同じ種類と判断し、`text=True` に `encoding="utf-8"` を明示した（SJIS 系のロケールではデコードが黙って文字化けしうるため。同じ種類の must-fix の全経路に当てる決まり）。
4. 足りなかった情報・検証手段: 非 UTF-8 ロケールがこの環境に入っていない（`locale -a` は C・POSIX と UTF-8 系だけ）ため、SJIS ロケールでの文字化けの経路は実測できず、構造上の解析（デコードとエンコードの両方を UTF-8 と決める）で閉じた。
5. 改善案: scripts/agent-sandbox/sandbox-shell のコメントに「`/home` の ro-bind により `$XDG_STATE_HOME` は sandbox から読み取り専用。ホストの見張りだけが書ける経路として使える」と 1 行あると、同じ判断を次の設計で即座に引ける。

使ったスキル: implement-issue。輸入スキルはなし

## 実装者メモ（round 5）

対象 head: 34689ac0ac99f17d17ef1680c6b14d17697af66e

1. 詰まった箇所: 公開先の書き込み可否の検査を根までたどると全ての場所が拒否になった。sandbox の `/` は uid 1000 所有の書き込み可の tmpfs（`mount` の出力で確認）で、実機で positive 経路が `writable-location` になって気づいた。マウントポイントはマウントに固定されていて rename・unlink が効かない（EBUSY）ため、探索は最初のマウントポイントで止めるのが正しい。モック python3 の仕込み方に続き、拒否の根拠をマウントの構造から説明する必要がある箇所で、この環境の mount の実測が役に立った
2. 曖昧だった文: 「運用で使う公開先をホストだけが更新できる場所に限定し」（codex round 4 レビュー）——コードで強制するか導入手順の運用に留めるかは書かれていないが、fail-closed の検査として実装できる（reader の側で公開先と親ディレクトリの書き込み可否を確かめる）ことに気づき、強制する側に取った。テストは書き込み可の tmp で動くため `PELWAY_BOARD_SNAPSHOT_ALLOW_WRITABLE` をテスト用のフックにした
3. 仮定と解釈: チェーンの探索をマウントポイントで止めてよい根拠は「マウントポイントのディレクトリは rename・unlink できない」ことで、これは Linux の vfs の規則。権限の写像の関係で自動テストの positive 経路（チェーン全体が書き込み不可で受け付ける）は作れず（`unshare -rm` では uid 0 になり os.access が常に真）、negative の粒度（ファイルのみ不可・親のみ不可・祖父母まで不可）と実機での検証（`~/.local/state` 配下の実ファイルで `writable-location` でないこと、`/tmp` に置くと `writable-location` になること）で当てた
4. 足りなかった情報・検証手段: userns の uid 写像を複数段にする仕込みがあれば positive の自動テストが書けるが、この環境では写像できるのは自分の uid 1 つだけ。runbook の導入手順に実機での確認を書いたため、運用でもカバーされる
5. 改善案: スクリプトのテストで tmp に作った連鎖ディレクトリのモードを 555 にすると trap の rm が片付けられず警告が出る。片付けの前にモードを戻すか、テスト用の一時ディレクトリの片付けに `chmod -R u+w` を挟む流儀があると共有点になる

使ったスキル: implement-issue。輸入スキルはなし

## 失敗の筋（実装前）

実装に入る前の段階で、差分の構造（見張りが gh でボードを読み、共有のファイルに書き出し、指揮役がそれを読む）から想定できた筋を round 1〜5 のメモとレビュー指摘から整理したもの。

- 失敗の筋: エージェント（指揮役と同じ uid・共有の置き場に書ける主体）が偽の書き出しを置くと、指揮役が偽のカード（着手の合図の Todo・停止の Backlog）を本物と読み、割り当てや停止を偽の情報に基づいて判断する
  確かめ方: reader の所有者の検査（`scripts/tests/board-snapshot-read.test.sh` 試験 8「foreign-owner」）、公開先の書き込み可否の検査（同試験 12「writable-location」の否定 3 本・試験 13 の読み取り専用マウントの肯定と書き込み可の対照）、runbook「ボード読み取りスナップショット」の真正性の扱い（承認は GitHub で読み直す）
- 失敗の筋: gh の応答が `--limit` で打ち切られると、欠けた一覧を完全なものとして公開し、載っていないカードを「無い」と読んでしまう
  確かめ方: writer の打ち切り検知と totalCount による取り直し（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 4）
- 失敗の筋: 型や番号の読めないカードを含む応答をそのまま公開すると、欠けたデータが指揮役の判断に届く
  確かめ方: カードの型検査の fail-closed（同試験 bad-item の系）、reader の型と有限性の検査（`scripts/tests/board-snapshot-read.test.sh` 試験 9・10）
- 失敗の筋: 書き出しの失敗（容量・符号化・置き換え）で前回分が消える・空になると、読めない期間にボードに依存する処理が止まる
  確かめ方: 取得失敗で前回分が残ること（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 2）、容量の上限（同試験 9）、非 UTF-8 ロケールの符号化（同試験 11）
- 失敗の筋: symlink や FIFO など通常でないファイルをたどって読むと、外のファイルを読む・待ち続ける
  確かめ方: reader の O_NOFOLLOW と S_ISREG（`scripts/tests/board-snapshot-read.test.sh` symlink・FIFO の系）、writer の O_EXCL|O_NOFOLLOW の一時ファイル（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 6）

## 失敗の筋（round 5 の最終）

round 5 の最終 head（d10c9f63）に対する筋と、その確かめ方。round 1〜5 のレビューで挙がった筋の最終的な対応を含む。

- 失敗の筋: 公開先が指揮役側から書き込み可の場所（`/tmp`・書き込み可の tmpfs 上のチェーンなど）にあると、エージェントが偽の書き出しを直接置ける・既存の書き出しを書き換えられる
  確かめ方: reader は公開先の fd のマウントの `ST_RDONLY` を主条件に確かめ、読み取り専用でないマウントでは公開先と親ディレクトリのチェーンを最初のマウントポイントまでたどり、書き込み可なら `broken`（detail `writable-location`）で拒否（`scripts/board-snapshot-read` :125-163、`scripts/tests/board-snapshot-read.test.sh` 試験 12 の否定 3 本・試験 13 の bwrap による読み取り専用マウントの肯定と書き込み可の対照。実機では `~/.local/state/pelway/` の実ファイルが通り `/tmp` の同じファイルが拒否されることを確認）
- 失敗の筋: 所有者の uid がずれた書き出し（置き場の取り違え・uid の写像の違い）を読むと、検査を通らないはずの入力が届く
  確かめ方: 所有者が実効 uid と違えば `broken`（detail `foreign-owner`）（`scripts/tests/board-snapshot-read.test.sh` 試験 8）
- 失敗の筋: 鮮度の判定に前の書き出しの `fetched_at` を使うと、偽の新しい書き出しで取り直しが止まり、偽のカードが本物に戻らない
  確かめ方: 間隔は見張りのプロセス内の記憶だけで管理し前の書き出しは読まない（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 5「forged fresh snapshot does not suppress refetch」）
- 失敗の筋: 未来の `fetched_at` を「新しい」と読むと、reader が拒否する書き出しを見張りが更新しなくなる
  確かめ方: 時計のずれ（60 秒）を超える未来は `broken`（`scripts/tests/board-snapshot-read.test.sh` 未来時刻の系・writer 側の future fetched_at is refetched）
- 失敗の筋: 書き込み途中の内容を指揮役が読む、symlink 経由で外のファイルを上書きする
  確かめ方: 一時ファイルは予測不能な名前を O_EXCL|O_NOFOLLOW で作り、書き切ってから置き換える（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 6・symlink の系）
- 失敗の筋: 非有限数（NaN・Infinity）や非 UTF-8 のバイト列で、標準 JSON として無効な書き出し・読めない出力になる
  確かめ方: writer は UTF-8 と `allow_nan=False` で書き、reader は有限性と型を検査（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 11、`scripts/tests/board-snapshot-read.test.sh` 試験 9・10）
- 失敗の筋: gh の応答の打ち切り・型の読めないカードで欠けた一覧を完全として公開する
  確かめ方: totalCount による取り直しと fail-closed（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 4・bad-item の系）
- 失敗の筋: 取得や書き出しの失敗で前回分が消え、読めない期間にボードに依存する処理が止まる
  確かめ方: 失敗した巡回は前の書き出しを残す（`scripts/tests/conductor-watch-board-snapshot.test.sh` 試験 2・9）、runbook「ボード読み取りスナップショット」の `ok: false` の手当て

## 実装者メモ（round 6）

対象 head: 9d1bdea134b80ed1976da6e79733e18fcd5afa50

1. 詰まった箇所: env の `-u` オプションは代入列の途中に置くとコマンド名として扱われる。テストで `env -u` を足した直後に試験 10 が落ちて気づき、オプションは先頭に置く形に直した（テスト自身が検知した）
2. 曖昧だった文: 「読み取り専用のチェーンを受け付ける肯定の自動試験」（GLM round 5）は `unshare` の例で書かれていたが、この環境では `unshare -rm` が権限の写像で使えず、bwrap の読み取り専用 bind マウントで代替した。bwrap 内で bind チェーンの親が読み取り専用になり、実機と同じ positive 経路（`ok: true`）を自動で確かめられた。書き込み可の bind の対照（`writable-location` で拒否）も同じ仕込みで足し、退化の両方向を CI で検知できる
3. 仮定と解釈: 「共有の置き場」を前提にした古い文の書き直しは、字面の消去ではなく多層防御の理由づけ（置き場の取り違え・マウントの写像のずれに備える）への置き換えと解釈した。理由が残ることで、次の読み手が同じ前提に戻らない
4. 足りなかった情報・検証手段: H31 の参照行は GLM の指摘（:148）と実際の節の位置（:146）がずれていたが、節の見出しの実位置に合わせた。行番号の参照は節の挿入でずれるため、今後は節名で引く方が保守しやすい
5. 改善案: なし

使ったスキル: implement-issue。輸入スキルはなし
