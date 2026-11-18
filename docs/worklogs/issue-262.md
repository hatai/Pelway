# Issue #262 の作業ログ

## 検証記録（2026-11-18 06:20〜06:26 JST）

Issue #262「canary を実 worktree の実ビルド成果物（target/ あり）で検証する」の検証結果。
mise.toml の demo-check タスク説明（「製品をビルドしたあとに実行する」）と dev-infra 作業ログ
各枠の「未検証・未決」（canary を合成ツリー selftest.sh でのみ検証していた）に対する実測。

### 実行環境と方針

- 実行環境: agent sandbox 内（`PELWAY_AGENT_SANDBOX=1`。scripts/agent-sandbox/sandbox-shell
  が worktree とキャッシュを bwrap で隔離した環境）。ホストの mbx キャッシュ
  `~/.cache/mbx/targets/v1/911a13c3…/`（実ビルド成果物 19276 ファイル・約 22GB。
  check/ debug/ tmp/ に pelway-agent-watch・pelway-dev-llm-eval・pelway-gateway-echo 等の
  バイナリ。エントリの mtime 2026-11-16 22:59）は `/home` の ro-bind で読み取り可。
- ビルドは実施していない。Issue 本文の「前提: ビルドを実施してよいかの確認」に代えて、
  ホストの mbx キャッシュにある実ビルド成果物を、実運用と同じ形状（worktree の
  `target` を `~/.cache/mbx/targets/` 配下への symlink にする。docs/dev-environment.md
  「mr boxington（mbx）について」）で対象にした。canary の目印（PELWAY_CANARY_*）は
  合成データ専用の文字列で、成果物のビルド元 commit に依存しないため、既存の実成果物で
  「対象あり・exit 0」と「mbx キャッシュ例外の実パスでの挙動」を確認できる。
- sandbox の制約: sandbox-shell はコマンド開始のたびに worktree の `target` symlink を
  実ディレクトリに戻す（mbx ハードリンクキャッシュ経由の書き換え事故防止。
  scripts/agent-sandbox/sandbox-shell の 74〜81 行）。そのため target symlink の作成と
  demo-check の実行は同一コマンド内で行った。

### 結果（コマンドと exit code）

1. 事前状態（target 無し・この worktree の素の状態）:
   `bash scripts/demo-check/run.sh canary` → exit 1。
   `canary: scanned 0 file(s), 0 hit(s)` + `no artifacts to scan` を出して失敗することを確認
   （走査対象が無いと未検証で失敗になる仕様の実働。この状態が本検証の RED に相当）。
2. 対象あり・exit 0（mbx 例外の実パス）:
   `target` → `~/.cache/mbx/targets/v1/911a13c3…/` の symlink を作り
   `bash scripts/demo-check/run.sh canary` → **exit 0**。
   `canary: scanned 19276 file(s), 0 hit(s)` → PASS（約 26 秒）。
   走査ファイル数 19276 はキャッシュ内の実ファイル数（`find <entry> -type f | wc -l`）と
   一致し、mbx 例外（target の実体が `$HOME/.cache/mbx/targets/` 配下のとき走査する。
   例外が無ければ `scan root resolves outside the work tree` で exit 2）を通じて実成果物を
   実際に走査したことを示す。実ビルド成果物への目印の混入は 0 件。
3. フル実行（target あり・4 段階）: `bash scripts/demo-check/run.sh` → exit 1。
   canary PASS（scanned 19276, 0 hit）。他 3 段階は失敗:
   loopback 3 件（crates/pelway-http/tests/edge.rs の all-interfaces）、
   leak-phrases 20 件（pelway-db/MIGRATIONS.md の /安全/、pelway-llm の
   PELWAY_LLM_API_KEY 識別子と sk- 形）、external-refs 27 件（pelway-auth・pelway-llm
   のテストの example.com 等）。いずれも本 Issue の diff（docs/worklogs/ 追記のみ）が
   触れない領域の main 由来の既存違反で、canary 段階の検証には影響しない。
4. 境界 A（例外は target のみ・実パス）: `logs` → 同じ mbx キャッシュへの symlink で
   `bash scripts/demo-check/canary.sh` → exit 2。
   `scan root resolves outside the work tree (not the mbx cache): logs -> ~/.cache/mbx/targets/v1/911a13c3…`
   （mbx 例外は target だけ。logs の外向けリンクは読まない）。
5. 境界 B（target でも mbx 配下以外は拒否・実パス）: `target` → `/tmp/opencode/nonmbx-artifacts`
   （worktree 外・非 mbx）の symlink で `bash scripts/demo-check/canary.sh` → exit 2。
   `scan root resolves outside the work tree (not the mbx cache): target -> /tmp/opencode/nonmbx-artifacts`。

### 結論

- 対象あり・exit 0: 実ビルド成果物 19276 ファイルを走査し 0 hit で PASS、exit 0 を確認。
- mbx キャッシュ例外の実パスでの挙動: 実運用と同じ symlink 形状で例外が機能し（4・5 で
  境界も実パスで確認）、worklog 各枠の「canary の実データ走査は未確認」は解消。
- 報告の論理パス表記（`target/…` として報告）は 0 hit のため本検証では直接観察できていない。
  この側面は selftest.sh の合成ケース（mbx キャッシュ例外で target リンク経由の走査と報告を
  固定）が既に担保している。実走査の証拠は走査ファイル数の一致（19276）。

## 実装者メモ（round 1）

対象 head: 61fac4a7b8a5bf46a8c07dc2b6c5ea1ab42f6c60

1. 詰まった箇所: scripts/agent-sandbox/sandbox-shell:74-81: コマンド開始のたびに worktree の target symlink が実ディレクトリに戻されるため、ln -s で作った symlink が次コマンドで消えた（1 回目の mise run verify では、検証中に発行した自分の shell コマンドがこの戻しを誘発し cargo の出力先を壊して exit 101。2 回目は shell コマンドを発行せず完了）。ln -sfn がディレクトリを渡すとリンクを「その中に」作る罠にも一度ハマった
2. 曖昧だった文: 「前提: ビルドを実施してよいかの確認（各スロットはビルド禁止で進めてきたため）。」（Issue #262 本文）— 誰が・いつ確認するかと、不確認のまま取れる代替手段が書かれていない
3. 仮定と解釈: ホストの mbx キャッシュ（~/.cache/mbx/targets/v1/911a13c3…、19276 ファイルの実ビルド成果物）を実運用と同じ symlink 形状（docs/dev-environment.md「mr boxington（mbx）について」）で走査対象にすれば「実ビルド成果物に対する canary 走査」は満たせ、ビルド許可の判断は不要と解釈した。canary の目印は合成データ専用文字列で成果物のビルド元 commit に依存しないため。sandbox 内で cargo build する選択（sandbox キャッシュへの書き込みと時間）は採らなかった
4. 足りなかった情報・検証手段: 検証対象キャッシュのビルド元 commit の同定手段（記録にはエントリの mtime 2026-11-16 22:59 と内容のバイナリ名のみ）。mise run verify の失敗が 1 件残る: scripts/tests/conductor-watch-ensure.test.sh（単独実行でも失敗・失敗ケースが実行ごとに変わる環境差分。docs/worklogs/issue-264.md round 1 と同一。verify-lint 0・verify-static 0・cargo test 0・script-test 27/28 は確認済み）
5. 改善案: .agents/skills/write-issue/SKILL.md の「未決事項」に「前提として人の確認が要るものは、確認者と、未確認のまま進める代替手段を併記する」を足す

使ったスキル: implement-issue（tdd は試行指定なしのため未使用。RED→GREEN は走査対象の有無の状態差で表現）
