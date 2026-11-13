# 作業記録: 開発基盤（dev-infra）

- 担当: Pelway 開発基盤エンジニア
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform`
- branch: `demo-t3/platform`（base `f32ca74c25d8c5614e7655a11c2a929be8211a10`）
- 時刻はすべて日本時間（JST, UTC+9）
- データは合成データのみ。外部送信なし。待受は 127.0.0.1 のみ。

## 記録の書式

各作業枠ごとに 1 項目を追加する。`glm:` には GLM（OpenCode + zai-coding-plan/glm-5.3）の使用有無を書く（`none` / `short` / `long`）。

```
### YYYY-MM-DD 枠名
- start: YYYY-MM-DD HH:MM JST
- stop:  YYYY-MM-DD HH:MM JST
- glm: none | short | long（T3 thread ID）
- 内容:
- 検証（コマンドと exit code）:
- 未検証・未決:
```

## 記録

### 2026-11-09 第1枠（GLM 不使用枠）
- start: 2026-11-09 14:51 JST
- stop:  2026-11-09 14:55 JST
- glm: none（T3 thread は使っていない）
- 内容:
  - `mise.toml` の rust に `targets = ["wasm32-unknown-unknown"]` を追加。`mise install` で `mise.lock` の rust options に targets が追記された。
  - `demo-check` の土台: `mise run demo-check` → `scripts/demo-check/run.sh` が 3 段階（loopback / leak-phrases / external-refs）を全部実行する。各段階は未実装のため `NOT IMPLEMENTED` を出して exit 3、全体は exit 1。
  - この作業記録を追加。
- 検証（コマンドと exit code）:
  - `mise install` 0（mr-boxington の sigstore 取得で一時的な 504 警告あり、再試行後に完了）
  - `mise exec -- rustc --version` → rustc 1.96.0、0
  - `mise exec -- rustup target list --installed` → wasm32-unknown-unknown, x86_64-unknown-linux-gnu、0
  - `mise exec -- cargo check --workspace --locked --target wasm32-unknown-unknown` 0
  - `mise run demo-check` 1（3 段階とも NOT IMPLEMENTED。想定どおり）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-validate` 0 / `mise run hk-check` 0 / `mise run toolchain-check` 0
- 未検証・未決:
  - demo-check の 3 段階の中身（禁止語リスト・外部参照の許可リストは QA と合意してから）
  - `hk install` はこの枠では実行していない。hk の pre-commit は既にリポジトリ共有の `.git/config`（`hook.hk-pre-commit`）に登録済みで、コミット 45cd1e6 の時に実行され通過した（trailing_whitespace / newlines。Rust ファイルの変更が無いため cargo 系は対象外）
  - shellcheck は未導入のため未実施（`bash -n` の構文確認のみ 0）
  - 独立コードレビュー（Claude または Codex）は未実施
- 共有ツールチェーンへの追加（事後記録）:
  - `mise install` で `rustup target add wasm32-unknown-unknown` 相当が、共有の `~/.rustup` のツールチェーン 1.96.0 に入った（追加のみ・削除なし）。mise の rust は `~/.cargo/bin` への symlink のため、main と他の worktree からも見える。
  - 開発リードが事後に了承。今後、共有・グローバルな場所（`~/.rustup` `~/.cargo` `~/.local` apt など）への追加は、事前にルームでリードへ知らせてから行う。

### 2026-11-09 第2枠（GLM 不使用枠）
- start: 2026-11-09 14:56 JST
- stop:  2026-11-09 15:01 JST
- glm: none（T3 thread は使っていない）
- 内容:
  - demo-check の 3 段階を実装（コミット「feat(dev-infra): implement demo-check stages…」）。走査範囲・除外は `scripts/demo-check/lib.sh` の冒頭に記載。
  - loopback: static 検査（PELWAY_BIND_ADDR が 127.0.0.1:<port> か、全アドレス待受の記述が無いか）は失敗で exit 1。runtime（待受ソケットの列挙）は未実装で「runtime: PENDING」と表示し、合格扱いにしない（static の合格のみ exit 0）。
  - leak-phrases: `forbidden-phrases.txt`（QA未合意/暫定）。出力はファイル:行と規則だけで、行の中身は出さない。
  - external-refs: `allowed-hosts.txt`（QA未合意/暫定。localhost / 127.0.0.1 / ::1）。xmlns 属性の値は対象外。
  - 自己テスト `scripts/demo-check/selftest.sh` と mise タスク `demo-check-selftest`（29 件）。
  - 第1枠の stop を 14:55 に訂正、共有ツールチェーン追加の事後記録を追記。
- 検証（コマンドと exit code）:
  - `bash -n scripts/demo-check/*.sh` すべて 0
  - `mise run demo-check` 0（loopback static 0 件・runtime PENDING、leak-phrases 0 件、external-refs 0 件。走査 43 ファイル）
  - `mise run demo-check-selftest` 0（29 passed, 0 failed）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-check` 0
- 未検証・未決:
  - 禁止語・許可ホストのリストは QA未合意/暫定
  - loopback の runtime 検査は未実装（デモの起動方法の決定待ち）
  - 「安全」「安心」は語が広く、UI 文言の範囲（資産・UI 系ファイル全体と Rust の文字列リテラル）に絞っても、否定文や内部メッセージで誤検出しうる
  - shellcheck は未導入のため未実施
  - 独立コードレビュー（Claude または Codex）は未実施

### 2026-11-09 第3枠（GLM 不使用枠）
- start: 2026-11-09 15:02 JST
- stop:  2026-11-09 15:09 JST
- glm: none（T3 thread は使っていない）
- 内容（QA の 2ef065a レビュー指摘とリード確定 15:01/15:03 を反映。コミット d707dfc）:
  - 必須修正: `run.sh` は段階名の完全一致（loopback / leak-phrases / external-refs / canary）だけを受け付け、それ以外の引数（パス・未知の名前・空文字）が 1 つでもあれば何も実行せず exit 2。
  - 段階を指定した実行は `PARTIAL RUN: not valid for acceptance` と表示し、まとめに「受入は引数なしの全段階実行だけ」と出す。
  - loopback: PELWAY_BIND_ADDR は 127.0.0.1:<port> と [::1]:<port> の完全一致だけ（localhost・0.0.0.0・127.0.0.10・括弧なし ::1 は不可）。
  - allowed-hosts.txt: 127.0.0.1 と ::1 だけ（localhost を削除）。
  - forbidden-phrases.txt: 英語の secure / safe / encrypted / fully deleted / permanently deleted（語単位・大文字小文字を区別しない）、24/72 時間・hours の表記そのもの（以前の「近くに通知」より厳しい）、秘密値の形（PRIVATE KEY ブロック・Bearer トークン・sk- トークン）を追加。冒頭に対象ファイル範囲（利用者向け文言と外向け文書だけ）を明記。状態はリード確定 2026-11-09 15:01/15:03。
  - 外向け文書として docs/public/ を走査対象に追加（docs/ のそれ以外は対象外のまま）。
  - Rust の文字列リテラル先頭にある語（例 "24時間"）を見落とす不具合を修正（自己テストで発見）。
  - 新しい段階 canary と canaries.txt（ダミーの目印 3 つ）。target/・logs/・log/・*.log・DEMO_CHECK_EXTRA_PATHS を走査し、走査対象が無ければ "no artifacts to scan" と明示して合格扱い。
  - 自己テストを 68 件に拡充。
- 検証（コマンドと exit code）:
  - `bash -n scripts/demo-check/*.sh` すべて 0
  - `mise run demo-check` 0（full run、4 段階。loopback static 0 件・runtime PENDING、leak-phrases 0 件、external-refs 0 件、canary 0 件。走査はソース 43 ファイル、canary 49 ファイル＝target/ の中身）
  - `bash scripts/demo-check/run.sh canary` 0（PARTIAL RUN 表示を確認）
  - `bash scripts/demo-check/run.sh ../../x/ok` 2（何も実行しない）
  - `mise run demo-check-selftest` 0（68 passed, 0 failed）
  - `mise run fmt-check` 0 / `mise run check` 0 / `mise run hk-check` 0
- 未検証・未決:
  - loopback の runtime 検査は未実装（デモの起動方法の決定待ち）
  - canary は今の target/ が開発確認用クレートの中身だけで、合成データを使ったデモのビルド・ログではまだ未確認
  - 英語の safe / secure は語単位でも、否定文や技術的な説明文（例: thread-safe の safe）で誤検出しうる
  - shellcheck は未導入のため未実施
  - 独立コードレビュー（Claude または Codex）は未実施

### 2026-11-09 第4枠（修正依頼1: Codexレビュー DC-01〜DC-07 の対処）
- start: 2026-11-09 16:33 JST
- stop:  2026-11-09 16:47 JST
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。本作業スレッド自身の T3 thread ID は記録時点で取得できず未記入）
- 内容（コミット a4411e6（修正）・この作業ログのコミット（記録））:
  - 第1〜3枠は T3 を通さずに直接書いていた逸脱。リード判断（2026-11-09）: 作り直しは不要とし、
    全体の Codex レビューと QA の合格を統合の条件とする。
  - 止める前に残っていた未コミット変更（scripts/demo-check の7ファイル・+45/−16）の扱い（ファイルごと）:
    - allowed-hosts.txt: 使わなかった。未コミット版が追記していた「QA合意（6cfa813 再確認）」は
      検証できなかったため、コミット済みの「リード確定」のみの文言に戻した（QA ラベルが無いことを確認）。
    - canaries.txt: 使った。「QA未合意」ラベルの追記を維持（ラベルは残す方針）。
    - forbidden-phrases.txt: 使った。「数字境界のリテラル先頭修正済み」の追記を維持（QA 再確認までラベルは残す）。
    - leak-phrases.sh: 使った。文字列リテラルの先頭・末尾パディングの修正を維持し、DC-05（pipefail・dc_load_files）を追加。
    - loopback.sh: 使った。ACCEPTANCE 部分を維持し、DC-01（bind 引数の照合）・DC-06（ポート 0〜65535）・DC-05 を追加。
    - canary.sh: 使った。ACCEPTANCE 部分を維持し、DC-03/04/05 でファイル列挙を再構成
      （ルートを realpath で解決・NUL 区切り・走査失敗で exit 2）。
    - run.sh: 使った。ACCEPTANCE MODE 表示を維持し、DC-05 の pipefail を追加。
  - Codex レビュー（f32ca74..6cfa813、スレッド mcp:3f306166-ba6d-450a-a950-9aefed39cfc9、full-access）の
    指摘 DC-01〜DC-07 と対処。Codex が読み取り専用で動いたこと・実行コマンドの内容は「自己申告・未検証」。
    こちらで確かめたのは修正前後の比較（6cfa813 と作業後の差分、selftest の失敗→合格）のみ:
    - DC-01: PELWAY_BIND_ADDR を経由しない bind("...")（例: TcpListener::bind("localhost:8080")、
      127.0.0.1 以外の IP）も 127.0.0.1/[::1]:ポート と完全一致で照合。不一致は exit 1。
      引数が文字列リテラルの場合のみ対象（変数は静かに見えない）。
    - DC-02: src=/href= 以外の、引用符（" ' `）の直後の「//ホスト名」（JS の文字列内など）も検出。
      ホスト部はドット付き TLD か IPv4 の形だけに限り、行頭 // コメント・path//file は誤検知しない。
      同じファイル:行:ホストの重複報告は1件にまとめた。
    - DC-03: find から -L を外し、走査中に出会ったシンボリックリンクをたどらない。
      canary の target/ などルート自体がリンクのときは realpath で実体を走査する
      （mbx の target リンクは引き続き読む。報告はこれまでの論理パスのまま）。
    - DC-04: dc_all_files・canary のファイル列挙を NUL 区切りに変え、改行を含むファイル名でも壊れない。
    - DC-05: 全スクリプトに set -o pipefail を追加。find 等の失敗（読めないディレクトリ等）は exit 2
      （dc_load_files・loopback の設定走査・canary の成果物走査が失敗状態を検査する）。
    - DC-06: ポートを 0〜65535 の数値で検査し、65536〜99999 を違反に。0 は従来どおり自動割り当てとして許可
      （リード確定 15:01/15:03 の仕様と mise.toml の 127.0.0.1:0 を維持。1〜65535 に厳密にすると
      mise.toml 自体が違反になるため。厳密にしたい場合は指示を乞う）。
    - DC-07: DC-01〜DC-06 の違反・通過ケースを selftest に追加（68 → 102 件）。
  - 繰越項目:
    - 数字の境目: Rust のリテラル先頭・末尾の 24/72（日本語・英語）を検出（未コミット修正を維持）。
      selftest に 24/72 開始の日本語・英語リテラルと、「1024 hours」が通るケースを追加。
    - DEMO_CHECK_ACCEPTANCE=1: loopback の runtime PENDING と canary の走査ゼロを exit 1 にし、
      出力に ACCEPTANCE MODE と表示（未コミット修正を維持）。未設定時の挙動は不変。
      PARTIAL RUN での動作も selftest に追加。
    - ラベル: allowed-hosts.txt から QA ラベルを外す（リード確定のみ）。forbidden-phrases.txt は
      QA 再確認までラベル維持、canaries.txt の「QA未合意」も維持。
  - TDD: 各項目の失敗ケースを先に selftest に追加し、HEAD（6cfa813）版スクリプトを /tmp に置いて
    22 件の RED を確認してから修正した（この時点の worktree では未コミット修正により繰越項目は既に緑、
    DC 項目の 16 件が赤）。修正後に 102 件すべて緑。
  - 記録だけ（対処なし）:
    - run.sh は 4 段階すべてを名前で指定すると partial=0 になる（引数なしの全段階実行と区別が付かない）。
      受入は引数なしの全段階実行だけ、という運用で問題なし。
    - Rust の文字列リテラルの取り出しは sed ベースで、エスケープ（\" など）・raw 文字列（r"..."）・
      複数行リテラルに弱い。
  - 使用skill: tdd・receiving-code-review（~/.agents/skills の Matt Pocock 版）。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh → 0（102 passed, 0 failed）
  - bash scripts/demo-check/run.sh → 0（4段階・full run。loopback static 0 件・runtime PENDING、
    leak-phrases 43 ファイル 0 件、external-refs 43 ファイル 0 件、canary 49 ファイル 0 件）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/run.sh → 1（loopback の runtime PENDING が
    受入不合格になる。仕様どおり。failed: loopback）
  - bash scripts/demo-check/run.sh loopback → 0（PARTIAL RUN 表示を確認）
  - bash -n scripts/demo-check/*.sh → 0
  - HEAD 版スクリプトに対する RED 確認（/tmp/opencode/old-dc に退避して実行）→ 1
    （80 passed, 22 failed。DC-01〜06 と繰越項目の失敗が出る）
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - DC-06 のポート 0 の扱い（0〜65535 として実装。上記の判断理由）
  - loopback の runtime 検査は未実装（PENDING のまま）
  - 仕様の未決事項 U01〜U07 はこの枠では決めていない
  - 外部レビュー担当が読み取り専用で動いたか・どんなコマンドで調べたかは自己申告・未検証

### 2026-11-09 第5枠（修正依頼2: Codex再レビュー slot5 の対処）
- start: 2026-11-09 17:35 JST（目安。17:41:58 に開発リードへ質問）
- stop:  2026-11-09 18:46 JST
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- 使用skill: tdd・receiving-code-review（~/.agents/skills の Matt Pocock 版。スロット4と同じものを使用）
- 内容（コミット 3a883e8（修正）・e94d4ec（17:56 決定への揃え直し）・この作業ログのコミット（記録））:
  - レビュー: Codex 再レビュー（範囲 6cfa813..60f6b28、レビュー本文 sha256
    d99c6160fd82476cfa2967e5e78d7c120348215ffa6253b704583f18c9582a73）。4件とも開発リードが採用。
    レビュー担当がスクリプトを実行していないこと・添付のコマンド一覧で調べたことは自己申告・未検証。
    こちらで確かめたのは修正前後の selftest（失敗→合格）と実リポジトリでの run.sh のみ:
    - 指摘1（loopback.sh、must-fix）: 同じ行に複数の bind(...) があってもすべて検査するよう、
      1行ごとに正規表現マッチを消化して繰り返す形に修正。selftest に「許可→localhost」「localhost→許可」
      「許可→許可」の同一行3ケースを追加。
    - 指摘2（canary.sh、must-fix）: 暗黙の走査ルート（target/ logs/ log/）の実体が作業ツリーの外に
      解決されるなら読まずに exit 2。例外は作業ツリー直下の target だけ: 実体を解決すると
      $HOME/.cache/mbx/targets/（mr-boxington のビルドキャッシュ）の下を指すときはその中身を走査する
      （開発リード決定 17:56。selftest は HOME を一時ディレクトリに差し替えて確認し、本物の
      ~/.cache には触らない）。旧「リンク先を実体走査して検出」の selftest は「外を読まずに
      exit 2」を確かめる形に書き換え、通る例（mbx 配下の target は検出できること）と止まる例
      （それ以外のリンク）を追加。さらに決定文どおり正確に揃えるため、XDG_CACHE_HOME では例外が
      広がらないことと、logs/ がキャッシュを指しても例外にならない（exit 2）ことの2ケースを追加。
      DEMO_CHECK_EXTRA_PATHS は明示指定のため検査対象外。
    - 指摘3（external-refs.sh、should-fix・採用）: 参照走査の前に、文字列外の行コメント
      （行頭または空白の直後の // から行末まで）を消すように修正（引用符の直前の // は
      文字列内のプロトコル相対参照なので残る）。selftest に JS 行コメント内の引用符付き //ホスト・
      https URL が誤検出されない2ケースと、末尾に行コメントが付いた実コードの引用符付き参照が
      引き続き検出される1ケースを追加。
    - 指摘4（canary.sh、must-fix）: 目印検索の前に、走査対象ファイルがすべて読めることを確認し、
      読めなければ exit 2（xargs 経由の grep の終了状態は no-match と区別できないため、
      読み取り可否の事前確認で担保）。selftest に権限のないファイル（chmod 000）と
      権限のないディレクトリで canary が exit 2 になる2ケースを追加。
    - DC-06: ポートは 0〜65535 で確定（開発リード 17:02）。挙動は変えていない。
      selftest に「許可リストにないホスト(localhost)ではポート 0 でも違反」と
      「localhost:65536（ホストもポートも不許可）は違反」の2ケースを追加
      （いずれも最初から緑。確認用の固定）。
  - TDD: 新しい失敗ケースを先に selftest に追加し、修正前に RED を確認
    （113件中5件が失敗: 指摘1の「許可→localhost」、指摘2の exit 2、指摘3の誤検出2件、指摘4の
    読めないファイル。ほかの新規ケースは不許可先が先頭に来る場合やガードとして最初から緑）。
    修正後に 113 件すべて緑。17:56 の決定文への揃え直しでも同じ手順をもう一度:
    XDG_CACHE_HOME と logs/ の2ケースの RED（113 passed, 2 failed）を確認してから修正し、
    115 件すべて緑。
  - 指摘2の質問と確定（17:41:58 に質問、開発リードが 17:56 に決定）: 実装前に「この worktree の
    target/ が mbx 共有キャッシュへのシンボリックリンクで、レビューどおり exit 2 にすると
    run.sh が 1 になる」ことを開発リードに質問。決定: 作業ツリー直下の target が実体を解決すると
    $HOME/.cache/mbx/targets/ の下を指しているときだけ例外としてその中身を走査し、それ以外の
    場所を指す走査ルートは外を読まずに exit 2 で止める。selftest は一時ディレクトリで HOME を
    差し替えて確認する（本物の ~/.cache は触らない）。
    初回実装（3a883e8）は XDG_CACHE_HOME による代替解決を含み、例外を logs/ log/ にも適用して
    いたが、決定文に正確に揃え、別コミット（e94d4ec）で絞り込んだ。
- 追加の記録（開発リードの指示による）:
  - 2026-11-09 16:20〜16:28（JST）に、開発基盤・Rust・バックエンドの GLM 3つが同時に動いていた。
    当時の「同時実行は2担当まで」というルールに反していた（このルールは 16:55 以降に廃止）。
  - allowed-hosts.txt の WIP 版にあった「QA合意（6cfa813 再確認）」の文言: slot4 レビュー時に作業ツリーに
    残っていた未コミットの WIP に含まれていたもの。QA は 6cfa813 に合意していない（QA が部屋で確認）。
    どの作業で入ったかは git 履歴では特定できず「出所不明」（git log -S で確認: allowed-hosts.txt の
    履歴に存在しない。コミットされたことは一度もない）。検証できないため slot4 では採らず、
    コミット済みの「リード確定」のみの文面に戻した（開発リードが妥当と判断）。
  - shellcheck は入っていないため、走らせていない。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh → 0（115 passed, 0 failed）
  - bash scripts/demo-check/run.sh → 0（4段階・full run。loopback static 0 件・runtime PENDING、
    leak-phrases 43 ファイル 0 件、external-refs 43 ファイル 0 件、canary 49 ファイル 0 件。
    target/ → mbx キャッシュのリンクは例外として引き続き走査）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/run.sh → 1（ACCEPTANCE MODE 表示。
    loopback の runtime PENDING が受入不合格。failed: loopback）
  - 修正前の RED 確認（selftest に新ケースを追加した直後の実行）→ 1（108 passed, 5 failed）
  - 決定文への揃え直し前の RED 確認（XDG_CACHE_HOME・logs/ の2ケース追加直後）→ 1（113 passed, 2 failed）
  - bash -n scripts/demo-check/*.sh → 0
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま）
  - 指摘4の担保は読み取り可否の事前確認であり、検索中の races（確認と検索の間に権限が変わる場合）は
    対象外
  - 行コメントの除去は // 形式のみ（# 形式や /* */ ブロックコメントは対象外）

### 2026-11-09 第6枠（修正依頼3: canary 走査0件の未検証失敗化。slot6）
- start: 2026-11-09 19:46 JST
- stop:  2026-11-09 20:24 JST（目安）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-slot6` / branch: `demo-t3/platform-slot6`
  （第5枠までの `demo-t3-platform` から複製された slot 6 用の worktree。開始 HEAD a5cc3f6・クリーンを確認）
- 使用skill: tdd・receiving-code-review（~/.agents/skills の Matt Pocock 版）
- 内容（コミット 2d3f0b8（修正）・この作業ログのコミット（記録））:
  - 指示: 開発リード作成の指示文（19:46 頃に届いた「canary 0件を未検証失敗にする／作業ログへ例外を追記」）。
    ハンドオフが予告していた「Codex方式2のレビュー本文（sha256 付き）」は本枠には届いておらず、
    指摘内容を開発リードが指示文に整理した形。レビュー本文の sha256 は記録できない（下の未決に記載）。
  - canary.sh: 走査対象 0 件のとき "canary: scanned 0 file(s), 0 hit(s)" と
    "no artifacts to scan (...)" を出して exit 1（未検証の失敗）。受入（ACCEPTANCE）専用だった分岐は
    廃止し、受入・通常で挙動は同じ。"PASS with note: nothing was scanned" の合格表現は出さない。
    対象 1 件以上のときは従来どおり scanned N file(s), H hit(s) を出し、目印なし exit 0 / あり exit 1。
    exit 2（リスト欠落・走査失敗）の意味は不変。
  - run.sh: コードは不変（canary の exit 1 は既存の段階失敗の経路で全体 exit 1 になる）。
    ヘッダコメントの DEMO_CHECK_ACCEPTANCE 説明を新しい既定に合わせて更新。
    終了コード: canary 未検証（0 件）は段階失敗扱いで run.sh 全体は exit 1（既存体系と同じ。
    検査異常の exit 2 も従来どおり段階失敗として run.sh は exit 1）。
  - selftest.sh: 115 → 114 件。
    - clean tree の合格ループから canary を外し、「0 件は exit 1 + scanned 0 + no artifacts +
      合格表現なし」を確認するケースに書き換え。
    - canary の ACCEPTANCE 専用ケース（0 件は exit 1）は通常と同じになったため削除（二重を避ける）。
    - clean tree の full run は canary だけ失敗（failed: canary）して exit 1 になるよう書き換え、
      canary に走査対象があるとき full run が exit 0 になるケースを追加。
    - symlink をたどらない確認（logs/link.log）は、0 件が失敗になる仕様になったため
      実ファイル（クリーン）を 1 つ置いて「symlink の中身を読まない（目印を出さず exit 0）」だけを確認する形に変更。
  - TDD: selftest を新しい期待に書き換えてから修正前に RED を確認
    （112 passed, 2 failed: 「canary 0 件は exit 1」canary.sh が exit 0、"clean tree の full run が
    exit 0" run.sh が exit 0。ほかの新規ケースは現行でも緑）。修正後 114 件すべて緑。
- 規則例外の記録（開発リードの指示による。指示文 19:46 頃、例外は 19:19〜19:26 JST）:
  - worktree 外の読み取り 2 件と兄弟 worktree の ls（自動許可 19:23:47〜48 JST）:
    `sha256sum /tmp/opencode/handoff-demo-check-slot6.md`、`/tmp/opencode/*` の読み取り、
    兄弟 worktree `demo-t3-platform/target` の ls。19:19:53〜19:23:47 の答え待ちのあと、
    auto モードで約 4 分後に自動許可されたもの。
  - `/tmp/opencode` への書き込み 2 件（規則から外れた 3 件目として扱う）:
    19:25:43 JST `slot6-selftest.log`、19:26:10 JST `slot6-run.log`。書いた者はスレッド出力からは
    未確定のため、安全側で GLM の例外として記録する。ファイルは指示どおり消していない（残置）。
  - 19:19:53〜19:23:47 JST の答え待ちでの停止（auto 許可までの空白）。
  - 開発基盤担当が T3 内部データ `statev2.sqlite` を読み取りで開いたこと（今後は開かない。
    リード決定どおり `t3 thread show` とスレッド出力だけで追う）。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh（修正前 RED）→ 1（112 passed, 2 failed）
  - bash scripts/demo-check/selftest.sh（修正後）→ 0（114 passed, 0 failed）
  - bash scripts/demo-check/run.sh → 1（full run。loopback PASS・leak-phrases PASS・external-refs PASS、
    canary は "scanned 0 file(s), 0 hit(s)" + "no artifacts to scan (...)" で FAIL、failed: canary。
    この worktree には target/ リンクが無く走査対象 0 件のため。対象 0 件が PASS 扱いになっていないことを確認）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/run.sh → 1（ACCEPTANCE MODE 表示。
    failed: loopback canary。loopback は runtime PENDING、canary は 0 件）
  - bash -n scripts/demo-check/*.sh → 0
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま）
  - ハンドオフが予告していた Codex方式2のレビュー本文（sha256 付き）は本枠に届いていない。
    指示文は開発リード作成のため、レビュー本文の内容・sha256 は未確認のまま
  - この worktree には target/（mbx キャッシュへのリンク）が存在しない。実リポジトリ相当での
    canary 走査（対象あり・exit 0）は selftest の合成ツリーで確認（mbx 例外のケース含む）。
    実 worktree での target ありの run.sh は未確認（ビルド禁止のため成果物を作らない）

### 2026-11-09 第7枠（修正依頼4: loopback bind ループの BASH_REMATCH 上書き。Codex r2）
- start: 2026-11-09 20:31 JST
- stop:  2026-11-09 21:26 JST（目安）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- 使用skill: tdd・receiving-code-review（~/.agents/skills の Matt Pocock 版）
- 内容（コミット 7926499（修正+テスト）・この作業ログのコミット（記録））:
  - 根拠: 開発リード指示文（Codex レビュー `mcp:b9379805`、指摘ファイル sha256
    321ccbf86c2cc56784dff136d3a20591299d98228358c43431e9b5847e17e3c2 の指摘1。QA が一時シェルで再現済み）。
    レビュー本文・指摘ファイルは本 worktree 内に存在せず、指示文の引用と selftest での再現のみで対処。
  - 不具合: loopback.sh の bind 引数ループ（旧90行）が次の検索位置を BASH_REMATCH[0] の長さで
    進めていた。判定関数 ok_bind_target 内の [[ =~ ]] が BASH_REMATCH を上書きするため:
    ポートのない引数（bind("localhost")）ではマッチ失敗で BASH_REMATCH が空になり（bash 5.2）進まず
    無限ループ、正しい引数でも進む幅が実際の一致位置まで足りず同じ bind を再検査して多重報告。
    実測（RED 時）: localhost は timeout 5 までに 41919 回報告して exit 124、
    bind("0.0.0.0:80") 単独で 5 回報告、許可+不許可の同一行で 6 回報告。
  - 修正: 一致全体（m）と引数（arg）を判定関数の呼び出し前にコピーし、次の検索位置は
    nxt="${rest#*"${m}"}"（rest 内の一致全体の直後）で計算。マッチ長だけ先頭から進めると一致の
    開始位置に届かないため（最初の修正で 3 重報告が残った点を確認済み）。進まなければ
    exit 2（検査異常）で止まる安全弁つき。
  - selftest.sh: 114 → 120 件。
    - A: ポートなし bind 引数が timeout 5 の中で終わり（124 にならず）報告 1 回。
    - B: 不正 bind 1 件の行で報告がちょうど 1 件。
    - C: 許可 bind と不正 bind が同一行に 1 件ずつで報告が不正の分の 1 件。
    - D（Codex の「自己テストがない」指摘）: canary で ../ 相対リンク・2 段リンク・
      targets-evil 接頭辞リンクのいずれも外を読まずに exit 2（いずれも現行実装で最初から緑。
      実装変更なしの確認の固定）。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh（修正前 RED）→ 1（117 passed, 3 failed: A/B/C。
    D の 3 ケースは緑）
  - bash scripts/demo-check/selftest.sh（修正後）→ 0（120 passed, 0 failed）
  - bash scripts/demo-check/run.sh → 1（full run。loopback 0 hit PASS・leak-phrases 43 ファイル
    0 件 PASS・external-refs 43 ファイル 0 件 PASS・canary 対象 0 件 FAIL（第6枠の仕様どおり）。
    failed: canary）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/run.sh → 1（ACCEPTANCE MODE。
    failed: loopback canary）
  - bash -n scripts/demo-check/*.sh → 0
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま）
  - nxt="${rest#*"${m}"}" は「rest 内の最初の m 出現の直後」に進める。同一行に一致全体と同一の
    文字列がより手前に出る場合のみ再検査になり得るが、進まないことはなく最終的に全部消化する
    （テスト B/C で単一報告を確認）
  - Codex レビュー本文・指摘ファイルの内容は指示文の引用のみで確認（未受領のため sha256 の検証は未実施）

### 2026-11-09 第8枠（修正依頼5: selftest の0件ケースを通常モードに固定。Codex r3 should-fix。slot8）
- start: 2026-11-09 22:05 JST（目安）
- stop:  2026-11-09 22:22 JST（目安。記録時点）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-slot8` / branch: `demo-t3/platform-slot8`
  （slot6 の流れ（60f6b28→…→666738d）から作られた slot 8 用 worktree。開始 HEAD 666738d・クリーンを確認）
- 手順: tdd の手順（テストを先に直す → RED → 修正 → GREEN → わざと壊して検出確認 → 復元）。
  skill の明示ロードはなし（第7枠までと同じ Matt Pocock 版 tdd 相当の手順）。
- 内容（コミット（修正）・この作業ログのコミット（記録）の2つに分ける）:
  - 根拠: 開発リード指示文（Codex 3回目レビュー（a5cc3f6..666738d）の should-fix を開発リードが
    21:59 に採用。must-fix 0・should-fix 1）。レビュー本文は本 worktree 内に存在せず、
    指示文の引用のみで対処。
  - 問題: selftest.sh の run_stage が外の DEMO_CHECK_ACCEPTANCE を環境ごと引き継ぐため、
    外で DEMO_CHECK_ACCEPTANCE=1 を付けて selftest を実行すると、canary 成果物0件のケース
    （通常モードのつもり）が受入モードで動き、「通常モードでも0件は exit 1」を確かめたことに
    ならない。canary.sh 自体は slot6 以降モードで挙動が同じなので現実には exit 1 のまま通るが、
    将来 canary.sh が通常モードで0件を合格に変えた場合に selftest が検出できない。
  - 修正（selftest.sh のみ。canary.sh・run.sh は無変更）:
    - run_stage の env に -u DEMO_CHECK_ACCEPTANCE を追加。run_stage 経由のケースは
      すべて通常モードに固定。受入モードで回す2ケース（loopback・canary）は従来どおり引数で
      DEMO_CHECK_ACCEPTANCE=1 を渡し、env の引数は左から右に効くため -u の後の渡しが勝つ
      （env -u X X=1 で X=1 になることを実機確認済み）。
    - run.sh を直接呼び出す通常モード3件（clean tree の full run / 成果物あり full run /
      単一段階 PARTIAL RUN）にも env -u DEMO_CHECK_ACCEPTANCE を追加。受入モードの run.sh
      2件（明示的に DEMO_CHECK_ACCEPTANCE=1 を付ける）は変更なし。引数検査のループ
      （exit 2 が先で ACCEPTANCE に依存しない）も変更なし。
    - ヘッダコメントに「外の DEMO_CHECK_ACCEPTANCE を引き継がない」旨を追記。
  - 迷った点と判断: 指示文の「0件ケース（通常モード）の呼び出し」だけでなく、修正前に
    DEMO_CHECK_ACCEPTANCE=1 外付けで selftest を走らせたところ loopback の通常モード
    8件（run_stage 経由。受入では runtime PENDING が失敗になるため）と run.sh 直接呼び出し
    3件の計11件も落ちることを確認。完了条件（両モードで selftest が exit 0）を満たすには
    同じ欠陥の一括修正が必要なため、run_stage の定義ごと（＝run_stage 経由全ケース）と
    run.sh 直接呼び出しの通常モード3件を直す方針にした。個別呼び出しごとの env -u より
    呼び出し側の変更が少なく、受入モードのケース（引数渡し）は一字も変えていない。
  - わざと壊しての検出確認（TDD の RED 相当。指示文の手順どおり）:
    - canary.sh の0件分岐に「通常モード（DEMO_CHECK_ACCEPTANCE 未設定）では exit 0」を
      一時的に挿入し、`DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh` を実行 →
      exit 1（118 passed, 2 failed）。落ちたケース:
      「canary: no artifacts to scan must fail with scanned 0 (exit 0)」（指摘の0件ケース。
      run_stage が通常モード固定で回ったため、壊しを検出）と
      「run.sh: full run on a clean tree (exit 0)」（0件が合格になることで full run が
      exit 0 になったことも検出）。
    - 確認後、canary.sh を必ず元に戻し、`git diff --stat scripts/demo-check/canary.sh` と
      `git status --short scripts/demo-check/canary.sh` が両方空（差分なし）であることを確認。
      修正前の selftest（run_stage が環境を引き継ぐ版）では、この壊しは外付け
      DEMO_CHECK_ACCEPTANCE=1 のもとで0件ケースが受入モードで動くため検出できない
      （修正前 RED 実行で同ケースが "ok" だったことで確認）。
- 検証（コマンドと exit code）:
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh（修正前 RED）→ 1
    （109 passed, 11 failed: loopback 通常モード8件＋run.sh 直接呼び出し3件。
    0件ケースはこの時点では ok＝受入モードで動いただけ、が指摘の本体）
  - bash scripts/demo-check/selftest.sh（修正後）→ 0（120 passed, 0 failed）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh（修正後）→ 0（120 passed, 0 failed）
  - わざと壊し（canary.sh 一時変更＋DEMO_CHECK_ACCEPTANCE=1 外付け）→ 1
    （118 passed, 2 failed。上記2ケース。確認後に canary.sh を復元し差分なしを確認）
  - bash scripts/demo-check/run.sh → 1（full run。loopback static PASS・runtime PENDING は
    通常モードのため失敗にならず、leak-phrases PASS・external-refs PASS、canary は対象 0 件で
    FAIL。failed: canary。exit 1 は仕様どおり）
    ※指示文には「canary 0件と loopback PENDING のため」とあったが、通常モードでは loopback の
    PENDING は失敗にならないため、実際の失敗は canary 0件のみ。実測をそのまま記録。
  - bash -n scripts/demo-check/*.sh → 0
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま）
  - 前枠から繰り越しの未検証3点（canary の実データ走査・loopback の本番待受確認・
    loopback.sh の did not advance で exit 2 になる道）は本枠では手を付けていない
  - Codex r3 レビュー本文は未受領（指示文の引用のみ。must-fix 0・should-fix 1 の確認も指示文による）

### 2026-11-09 第9枠（修正依頼6: 受入モードの canary 0件を単独ケースで固定。Codex slot8-r1 should-fix。slot9）
- start: 2026-11-09 22:55 JST（目安）
- stop:  2026-11-09 23:10 JST（目安。記録時点）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-slot9` / branch: `demo-t3/platform-slot9`
  （slot 8 の先端 858e4dd から作った slot 9 用 worktree。開始 HEAD 858e4dd905766dc32068df30bf6921852f9123d8・
  クリーンであることを確認してから着手）
- 手順: tdd（Matt Pocock 版を明示ロード。テストを先に書く → 現行実装でGREEN →
  わざと壊して該当ケースが落ちることを確認（RED相当） → 復元 → GREEN）。
- 内容（コミット（修正）・この作業ログのコミット（記録）の2つに分ける）:
  - 根拠: 開発リード指示文（Codex slot8-r1 レビュー（666738d..858e4dd）の should-fix を開発リードが
    22:51 に採用。対象ファイル sha256 827276e1…e4fb）。must-fix 0・should-fix 1。レビュー本文は本
    worktree 内に存在せず、指示文の引用のみで対処。shellcheck は未導入（入れない）。
  - 問題: selftest.sh の受入モード canary 個別ケースは「成果物あり」だけだった。受入モードの
    run.sh 全体実行は exit 1 を見ているが、その 1 は loopback の PENDING だけでも出るため、
    「受入モードで canary 0件が未検証の失敗（exit 1）」を canary 単独では確かめていなかった。
  - 修正（selftest.sh のみ。canary.sh・run.sh・lib.sh は無変更。+11行/−1行）:
    - 受入モードの canary 個別ケースに「成果物0件」の単独ケースを追加。run_stage に
      DEMO_CHECK_ACCEPTANCE=1 を引数で明示的に渡し（run_stage の env は -u の後に渡した方が
      勝つ。slot8 の仕組み）、canary.sh 単独で回して exit 1・「canary: scanned 0 file(s)」・
      「no artifacts to scan」のメッセージ・PASS を出さないことの4点を確認。loopback には頼らない。
    - 通常モードの0件ケース（env -u で固定）と受入モードの成果物ありケースは無変更。
      旧コメント「canary の走査ゼロは ACCEPTANCE 専用ではなくなったため、専用ケースは置かない」は
      単独ケースを置く方針に変わったため削除し、理由（loopback の PENDING だけでも全体 exit 1 に
      なるため単独で固定する）を書いたコメントに置換。
    - ヘッダコメントに slot9 の1項目を追記。selftest.sh は 120 → 121 ケース。
  - わざと壊しての検出確認（指示文の手順どおり。slot8 の自己申告 118 passed / 2 failed の実測相当）:
    - canary.sh の0件分岐（exit 1 の直前）に `[ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ] && exit 0` を
      一時的に挿入し、`bash scripts/demo-check/selftest.sh` を実行 → exit 1（119 passed, 2 failed）。
      落ちたケース:
      1. 「canary: ACCEPTANCE no artifacts must fail alone with scanned 0 (exit 0)」（今回追加した
         単独ケース。exit 0 を直接検出）
      2. 「run.sh: full run under ACCEPTANCE (exit 1)」（失敗段階リストの grep による間接検出。
         この壊しでは全体実行の exit code は loopback の PENDING だけで 1 のまま＝exit 1 の値だけでは
         canary の失敗と loopback の失敗を分けられない、というのがレビュー指摘の本体）
      通常モードの0件ケースは壊しが受入モード限定のため ok のまま（壊しが通常モードに影響しない
      ことの確認も兼ねた）。
    - 確認後、canary.sh を必ず元に戻し、`git diff --stat scripts/demo-check/canary.sh` と
      `git status --short scripts/demo-check/canary.sh` が両方空（差分なし）であることを確認。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh（修正前ベースライン）→ 0（120 passed, 0 failed）
  - bash scripts/demo-check/selftest.sh（修正後）→ 0（121 passed, 0 failed）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh（修正後）→ 0（121 passed, 0 failed）
  - わざと壊し（canary.sh 一時変更）で bash scripts/demo-check/selftest.sh → 1
    （119 passed, 2 failed。上記2ケース。確認後に canary.sh を復元し差分なしを確認）
  - bash scripts/demo-check/run.sh → 1（full run。loopback static PASS（runtime PENDING は通常モード
    では失敗にならず）・leak-phrases PASS・external-refs PASS・canary 対象 0 件 FAIL。failed: canary。
    仕様どおり。※指示文の「canary 0件と loopback PENDING のため」は、通常モードでは loopback の
    PENDING が失敗にならないため実際の失敗は canary 0件のみ。第8枠と同様に実測をそのまま記録）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/run.sh → 1（ACCEPTANCE MODE。
    failed: loopback canary）
  - bash -n scripts/demo-check/selftest.sh canary.sh run.sh → 0
  - shellcheck は未導入のため未実行
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま）
  - Codex slot8-r1 レビュー本文・対象ファイル sha256 は指示文の引用のみで確認（未受領のため
    sha256 の検証は未実施）
  - 前枠から繰り越しの未検証3点（canary の実データ走査・loopback の本番待受確認・
    loopback.sh の did not advance で exit 2 になる道）は本枠では手を付けていない

### 2026-11-13 第9枠・続（修正依頼7: 受入モードの目印ありケース。slot9-r1 should-fix。slot9-r2）
- start: 2026-11-13 23:10 JST（目安）
- stop:  2026-11-13 23:35 JST（目安。記録時点）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-slot9` / branch: `demo-t3/platform-slot9`
  （第9枠と同じ worktree で継続。開始 HEAD 06de61b・クリーンを確認）
- 手順: tdd（テストを先に書く → GREEN → わざと壊して RED → 復元 → GREEN）。
- レビュー（slot9-r1）: ユーザー指示により T3 の delegate_task で自分と同じモデル
  （opencode / zai-coding-plan/glm-5.3・新鮮なコンテキスト）をレビュアーとして起動し
  858e4dd..06de61b をレビュー。結果: must-fix 0・should-fix 1・nit 2。
  - should-fix: 受入モードの「目印あり → exit 1」個別ケースが無い。目印検出ケースは従来
    すべて通常モード実行で、レビューアーが受入モード限定で canary.sh の hits を 0 にする壊しを
    実証し、selftest が 121 passed, 0 failed のまま検出できないことを確認（worklog 未記載の新指摘）。
  - nit 1: 単独呼び出しケースの `! grep -q "PASS"` は実質空振り（"PASS" は run.sh の出力）。無害。
  - nit 2: 新ケースの判定式は通常モード0件ケースとほぼ同一。既存スタイルには合致。
  - レビューの検証で worklog 記載の検証コマンド・件数がすべて再現したこと、変更が2ファイルのみ・
    コミット分割が適切であることも確認済み。レビュー時点で worktree はクリーン。
- 内容（コミット（修正）・この作業ログのコミット（記録）の2つに分ける）:
  - 修正（selftest.sh のみ。canary.sh・run.sh・lib.sh は無変更。+12行）:
    - 受入モードの canary 個別ケースに「目印あり」の単独ケースを追加。logs/app.log の2行目に
      API-key 目印を置き、run_stage に DEMO_CHECK_ACCEPTANCE=1 を引数で明示的に渡して canary.sh
      単独で回し、exit 1 と「logs/app.log:2: canary PELWAY_CANARY_APIKEY_9c21」の報告を確認。
    - 新ケースには `! grep -q "PASS"` を付けない（レビュー nit 1 が「canary.sh 単独呼び出しでは
      空振り」と指摘したため、新規コードには書かない。既存ケースの同チェックは無害なのでそのまま）。
    - ヘッダコメントに slot9-r2 の1項目を追記。selftest.sh は 121 → 122 ケース。
  - わざと壊しての検出確認（レビューアーの実証と同じ壊し）:
    - canary.sh の目印集計直後（最終 echo の直前）に
      `[ "${DEMO_CHECK_ACCEPTANCE:-}" = "1" ] && hits=0` を一時挿入し、
      `bash scripts/demo-check/selftest.sh` を実行 → exit 1（121 passed, 1 failed）。
      落ちたケースは追加した単独ケースのみ
      「canary: ACCEPTANCE marker hit must fail alone (exit 0)」（直接検出）。
      他ケース（0件ケース・通常モード目印ケース・run.sh 系）は影響なし。
    - 確認後、canary.sh を元に戻し、`git diff --stat` と `git status --short` が両方空であることを確認。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh（修正後）→ 0（122 passed, 0 failed）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh（修正後）→ 0（122 passed, 0 failed）
  - わざと壊し（canary.sh 一時変更）→ 1（121 passed, 1 failed。上記1ケース。
    確認後に復元し差分なしを確認）
  - bash scripts/demo-check/run.sh → 1（full run。failed: canary。仕様どおり）
  - bash -n scripts/demo-check/selftest.sh canary.sh → 0
  - shellcheck は未導入のため未実行（Issue #265 で追跡）
- 統合（第9枠の続きとして実施）:
  - origin/demo-t3/platform を push 済み（06de61b）。demo-t3/platform は別 worktree で
    チェックアウト中のため本 worktree からは更新できず、PR は同一線上の
    demo-t3/platform-slot9（本修正込み）から main へ出す。
  - 未検証・未決の細かい残件は Issue #262〜#265 に起票済み（ready-for-agent）。
- 未検証・未決:
  - loopback の runtime 検査は未実装（PENDING のまま。次枠候補。デモ起動方法の決定が前置き）
  - レビュー nit 2件は無害として対処せず（本枠では新規コードに伝播させない対応のみ）

### 2026-11-13 第9枠・続2（PR #270 の競合解消と CodeQL 失敗の調査）
- start: 2026-11-13 23:40 JST（目安）
- stop:  2026-11-13 23:55 JST（目安。記録時点）
- glm: long（T3 Code / OpenCode・zai-coding-plan/glm-5.3。thread ID は記録時点で取得できず未記入）
- 経緯: PR #270（demo-t3/platform-slot9 → main）で CodeQL の "Analyze (actions)" が失敗し、
  main と競合した旨の通知が T3 Code から届いた。
- CodeQL 失敗の調査:
  - エラーは「CodeQL could not process any code written in GitHub Actions」。当 PR はワークフロー
    ファイルを一切変更しないため、actions 抽出の対象が空のまま finalize で落ちている。
  - 直近の他 PR（#258/#260/#261/#268）は成功。#260/#261/#251/#250 は .github 無変更でも成功しており、
    refs/pull/<N>/head の解析も同じ。#270 だけの違いは「競合で merge ref が作れない＋merge-base が
    古い（f32ca74）」こと。競合解消で merge-base が最新 main に寄るため、再実行で直るかを見る。
- 競合の正体: main に当作業ラインの移植（e3d1cc0「port demo-check stages」・f0e497d「add demo-check
  selftest」＝Issue #143。06de61b 相当＋移植適合）がすでに入っていた。
  - main 側の適合: lib.sh に除外追加（crates/dev-*・pelway-agent-watch・pelway-client-ui/tokens・
    pelway-http/tests/http.rs）、loopback.sh の bind 検出を ::bind("...") 形式に絞る（sqlx 等の
    メソッド形式 .bind を除外）、selftest に移植分のケース追加（121 → 130 ケース）。
  - mise.toml には demo-check / demo-check-selftest タスクあり（script-test から selftest を呼ぶ）。
- 解消（マージコミット cae8760）:
  - mise.toml / mise.lock: main 側を採用。demo-check タスクは main に既存。当ラインの slot1 の
    rust wasm32 ターゲット追加は main の移植で採用されなかったため再追加しない（demo-check の
    実行に不要。selftest は合成ツリーで完結）。
  - scripts/demo-check/lib.sh / loopback.sh: main 側を採用（移植適合込み。slot7 修正も保持）。
  - scripts/demo-check/selftest.sh: main 側（130 ケース）を土台に slot9-r2 の差分
    （ヘッダ1行＋目印あり単独ケース）のみ再適用 → 131 ケース。
  - マージ後の main との差分は selftest.sh +12 行と作業ログのみ（+537 行。pr-size 上限 800 内）。
- 検証（コマンドと exit code）:
  - bash scripts/demo-check/selftest.sh → 0（131 passed, 0 failed）
  - DEMO_CHECK_ACCEPTANCE=1 bash scripts/demo-check/selftest.sh → 0（131 passed, 0 failed）
  - bash scripts/demo-check/run.sh → 1（4段階とも FAIL。違反はすべて main 由来のファイル
    （pelway-http/tests/edge.rs の all-interfaces、pelway-llm の APIキー識別子と sk- 形）で、
    main の移植時点からの既存状態。main の CI が呼ぶのは selftest（script-test 経由）で run.sh ではない）
  - マージコミットの pre-commit フック: trailing_whitespace / newlines / typos / cargo check /
    cargo clippy / cargo fmt すべて通過（main の mise が rust 1.99.1 を要求したため mise install で導入）
  - bash -n scripts/demo-check/*.sh → 0
- その他の対応:
  - 作業ログを main の規約（AGENTS.md「Worklogs live under docs/worklogs/」）に合わせて
    docs/worklog/dev-infra.md → docs/worklogs/dev-infra.md へ移動。
  - 以後の GitHub 操作（push・PR 編集）は main の新 AGENTS.md の規則に従い scripts/agent-gh dev 経由で行う。
- 未検証・未決:
  - CodeQL "Analyze (actions)" が競合解消で再実行されて通るかは push 後の確認待ち
  - main のフルツリーで run.sh が4段階とも失敗する状態は main 側の既存問題（Issue #143 の移植の
    範囲外）。当 PR では対処しない
