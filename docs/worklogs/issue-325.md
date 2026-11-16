# 実装者メモ: Issue #325

- Issue: [#325](https://github.com/hatai/Pelway/issues/325)
- PR: [#359](https://github.com/hatai/Pelway/pull/359)
- 対象 head: aa8b181（実装コミット・署名付け替え後）

## 変更 ↔ 一次記録の引用 ↔ 期待する変化

| 変更 | 根拠（一次記録） | 期待する変化 |
|---|---|---|
| `scripts/agent-sandbox/sandbox-shell`: `/etc/resolv.conf` が symlink のとき `readlink -f` の実体を正本パスに `--ro-bind`（`$home` 配下を指す場合は除く） | 実環境での再現: `/etc/resolv.conf -> /mnt/wsl/resolv.conf`（WSL）。sandbox は top-level を個別マウントし `/mnt` を含めないためリンク切れになり、`lookup api.github.com on [::1]:53 ... connection refused` で DNS が死ぬ。sandbox 内での `getent hosts api.github.com` が成功することが受け入れ条件 | `/mnt/wsl/resolv.conf` が ro-bind され symlink が解決し、DNS が通る。`/mnt/c` 等の Windows 側は見えない |
| 同: sandbox 専用キャッシュ `~/.cache/agent-sandbox` を作成し rw-bind、`XDG_CACHE_HOME` に設定。作れないネスト実行では `/tmp/pelway-sbx-cache-<uid>` に退避 | 実環境での再現: `~/.cache/mbx` は ro で `mbx[error]: Read-only file system (os error 30)` と lock 取得失敗、`hk WARN failed to write cache file: /home/hatai/.cache/hk/...`。`MISE_RUST_MR_BOXINGTON=0` はコマンドラッパー生成済み環境では実行時に効かず cargo は mbx を通る（mise の生成 shim が固定化するため）。`XDG_CACHE_HOME=/tmp/...` の実測で mbx の lock・hk の書き込みが通ることを確認 | mbx・hk・mise・pkl のキャッシュが sandbox 専用ディレクトリに移り、verify・commit 時フックが lock/書込エラーなく動く。ホストの `~/.cache/mbx`（ハードリンクキャッシュ、Issue #151 の設計で非公開）は見せないまま |
| 同: `PELWAY_AGENT_SANDBOX=1` を env に追加 | Issue AC:「ホスト側専用テストは検出と理由を出して skip してよい」。sandbox 内から `agent-sandbox.test.sh` を実行するとフィクスチャ（`~/.t3/worktrees`・`$HOME` 直下）を作れず失敗するため、判定用の目印が必要 | ホスト専用テストが sandbox 内を確実に識別できる |
| `scripts/tests/agent-sandbox.test.sh`: 先頭で `PELWAY_AGENT_SANDBOX` を検出して理由を出して `exit 0`。検査追加: `XDG_CACHE_HOME` の向き先と書込可、ホスト `~/.cache/mbx`・`~/.cache/hk` の read-only、mbx 経由 cargo の lock 成功（`cargo new`+`cargo check`）、DNS 解決（`getent hosts api.github.com`）、resolv.conf symlink 先の到達、`/mnt/c` 不在 | Issue AC:「該当の挙動を検査する scripts/tests/agent-sandbox.test.sh の検査項目が、sandbox 外の `mise run script-test` で合格」 | sandbox 外の CI で新規検査が走り退行を検出。sandbox 内では理由付き skip で verify が exit 0 になる |
| `docs/dev-environment.md`: writable 一覧・env 一覧・確認手順・mbx 節・記録欄を実態に合わせて更新 | Issue AC:「docs/dev-environment.md の記載が実態と合う」 | 文書と実装の齟齬がない |

## 採用しなかった案（Issue の選択肢との対応）

- `/mnt/wsl` 全体の ro-bind: `/mnt` をマウントしない設計意図（`/mnt/c` 等の Windows 側ファイルを見せない）を広く崩すため、リンク先ファイル単体の ro-bind に留めた
- `~/.cache` 全体の writable 追加 / 必要な cache dir の列挙追加: Issue #151 で `~/.cache/mbx` のハードリンクキャッシュ経由の書き換え経路を意図的に閉じた設計と矛盾するため、sandbox 専用ディレクトリへの `XDG_CACHE_HOME` リダイレクトにした（列挙すると mbx だけでなく hk 等も必要になり専用 dir 1 つの方が単純）
- `MISE_RUST_MR_BOXINGTON=0` の実効化（wrapper 再生成等）: 生成済み shim が環境を固定化するため外部からの実効化は脆い。cargo が mbx を通ることを前提に向き先を変える方が確実。変数は意図の宣言として残す
- ネットワーク allowlist 化: Issue #151 で開放のままと明示されている
- review agent の sandbox（`claude_review` 等）への適用: Issue 範囲外（#305・#306）

## 検証（実施記録）

検証はこの実装スレッド自身（agent-sandbox 内）から、修正済み `sandbox-shell` をネスト起動して実施した。ネストでは `$HOME/.cache` が read-only のため `XDG_CACHE_HOME` は `/tmp/pelway-sbx-cache-1000` への退避経路を通った（退避ロジックの実動確認にもなる）。DNS は本スレッドの level-1 namespace に `/mnt/wsl/resolv.conf` を seed し、ネスト側の新コード（symlink 実体の ro-bind）で解決させた。

- `sandbox-shell -c 'mise run verify'` → `VERIFY_RC=0`。ログ内に `ERROR`/`FAILED`/`Read-only file system` 0 件、`test result: ok` 81 件。`[script-test] SKIP: agent-sandbox の内側（ホスト専用テスト: ...）` が出力され rc=0（追加環境変数なしで exit 0、AC1・AC2）
- `sandbox-shell -c 'getent hosts api.github.com'` → `20.27.177.116 api.github.com`、rc=0。`ls /mnt/c` → `No such file or directory`。`findmnt -T /mnt/wsl/resolv.conf` → tmpfs `ro` の bind を確認（AC3）
- `sandbox-shell -c 'mise run hk-check'` → `failed to write cache` 警告 0 件、全ステップ ✔（AC4）
- `sandbox-shell -c 'cargo test -p pelway-agent-watch --test once unwritable'` → ok（chmod 非依存の環境でも権限が効くことを確認）
- `bash -n` on `sandbox-shell` / `agent-sandbox.test.sh` → ok
- `mise run pr-size` → +81 / -15（上限内）
- sandbox 外での `mise run script-test` 通し実行: 指揮役がホストで `bash scripts/tests/agent-sandbox.test.sh` を実行し `PASS: agent-sandbox`・exit=0（Issue #325 コメント 6905353781 の記録、受け入れ条件 2 の証跡）。なお CI の `test (cargo test)` ジョブ（run 43743202520・job 132900858476）は bwrap 不在で `SKIP: bwrap not installed` になり、CI の pass は当該検査の実行を意味しない（レビュー指摘の選択肢 b を検証し不成立を確認）。テストの新規検査項目はネスト sandbox でも個別に等価確認済み（XDG 書込可・DNS・skip）
- ネストで通し実行を試した記録: `sandbox-shell -c` で `env -u PELWAY_AGENT_SANDBOX bash scripts/tests/agent-sandbox.test.sh` → プラグイン検査が `~/.t3/worktrees` 直下のフィクスチャ（`sbx-plug-test`・`sbx-plug-out`）を作れず失敗し、ホスト専用であることを確認。併せて `~/.cache/agent-sandbox` が存在するが read-only のとき退避しない不具合を見つけ `-w` 検査に直した

## 詰まった箇所・仮定

- 検証中に一度 `mise run verify` が `unwritable_state_does_not_repeat_events` で FAILED したが、これは検証用に使った `unshare -rm` の uid 0 マッピングの副作用（chmod が効かない）で、実 sandbox（uid 1000 マッピング）では当該テスト単体が pass することを確認済み。修正版 sandbox 内での再実行では全件 pass
- `mise run` は毎回 rustup が channel manifest を取りに行くため、sandbox 内の全コマンドで DNS が必須。今回の resolv.conf bind でまとめて解消される
- 仮定: `/etc/resolv.conf` が実ファイルのホストでは `/etc` の ro-bind に含まれるため今回のコードパスは発火しない（`[[ -L ]]` ガード）
- `~/.cache/agent-sandbox` はホスト FS 上に実在する新規ディレクトリ。sandbox を出た後も残るが、内容は mbx/hk/mise/pkl のキャッシュのみで秘匿情報は含まない

## 実装者メモ（round 2）

対象 head: 1fb4205（must-fix 対応コミット・署名付け替え後）

1. 詰まった箇所: `docs/dev-environment.md` の「確認手順の結果（記録欄）」— round 1 でホスト通し実行をしていない新規検査を「合格」・未来日付（2026-11-18、コミットは 2026-11-15）で書いてしまい、claude レビューで must-fix になった。実態（ネスト sandbox で個別確認済み・ホスト通し未実施）と実日付に修正
2. 曖昧だった文: 「`docs/dev-environment.md` の「実装担当の sandbox（agent-sandbox）」と「mr boxington（mbx）について」の記載が実態と合う → 差分で確認」（Issue #325 受け入れ条件 5）— 「記載」に記録欄の実績行が含まれるかが曖昧で、実績欄に検査の新規内容を入れるかの判断が必要だった
3. 仮定と解釈: round 1 では記録欄を実績欄と読み、新規検査の記録を入れてよいと解釈した。ただし「合格」と確定日を書くのはホスト通し実行の記録があってからというのが正しい扱い（レビュー指摘で確定）。ホスト通し実行は実装スレッドから起こせないため hatai に依頼中
4. 足りなかった情報・検証手段: ホストでの `bash scripts/tests/agent-sandbox.test.sh` 通し実行の結果（sandbox 内では `~/.t3/worktrees`・`$HOME` 直下にフィクスチャを作れず、CI は bwrap 不在なら skip する）
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 5・6 の間か止まり方の節に「記録欄・実績表には実行した事実だけを書く。未実施の検査を『合格』と書かない」という一文を足す（今回の round 1 で実際に踏んだ形）

使ったスキル: implement-issue
