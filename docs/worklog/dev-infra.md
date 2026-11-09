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
