# Issue #689 の作業ログ

## 実装者メモ（round 1）

対象 head: 45da7c8326433674e8aa717e6a60c329e056a8b5

1. 詰まった箇所: 調査自体: `/dev/kvm` は実装担当の sandbox（bwrap）の内側から見えない（sandbox の `/dev` は最小構成で、`/dev/kvm` を含まない）。「`/dev/kvm` が無い＝ホストに無い」と書き違えないよう、`lsmod` の `kvm_amd`（モジュール一覧は名前空間で分かれない）と Issue 本文のホスト側記録（18:57 JST）を併記する形にした。
   - `mise run verify`: `.agents/skills/verify-pelway/verify-run.test.sh` が 6 件失敗した。差分（文書 2 件の追加のみ）を stash しても再現し、CI の main の `mise run verify` は通る（2026-11-24T10:01Z の check_runs）ため環境起因。切り分けると、sandbox 内で `/tmp` の所有者が `nobody`（uid 65534、`stat -c '%u %a' /tmp` → `65534 1777`）に見えるため `dir_chain_safe` の「所有者が自分か root」の条件で落ちる。フォロー Issue #692 を起票した。
2. 曖昧だった文: 「6 つの観点のそれぞれについて、bwrap 方式と `sbx` の両方の欄があり、`sbx` の欄には出典（公式文書の URL）か「minisforum で確認（コマンドと日時 JST）」が付いている」（Issue #689 の受け入れ条件）。bwrap 側の欄の出どころの書き方は指定が無かったため、リポジトリ内のパス（`scripts/agent-sandbox/sandbox-shell`・`docs/dev-environment.md`）を出どころとして書いた。
3. 仮定と解釈: 「T3 の worktree スレッド・opencode・Devin CLI の起動の仕方に乗るか」は、「T3 が ACP でエージェントを起動する構成のまま `sbx` の内側にエージェントを入れられるか」と解釈した。「`sbx run <agent>` で T3 を介さず直接エージェントを動かす運用への置き換え」は対象外と読み、候補にも挙げなかった（Issue の「対象外」に T3 の設定変更があり、運用の置き換えはこの Issue の調査範囲を超えるため）。`sbx` の Windows 版経路（`\\wsl.localhost` のワークスペース）は、sbx-releases #598・#900 の障害報告から候補にしなかった。
4. 足りなかった情報・検証手段: 公式文書に FAQ・credentials・usage の URL が 2 系統（`/ai/sandboxes/faq/` と `/ai/sandboxes/configuration/faq/` 等）に見え、初回の取得は 404 になった。正しい URL は一覧ページから辿って確かめた。git worktree をマウントしたとき共有 git dir まで git が動くか、追加 workspace のマウントをネストできるかは、文書に記載が無く未確認のまま（文書に「未確認」と記入）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「比較表を求める条件では、各欄の出どころの書き方（URL・コマンドと日時・リポジトリ内のパスのどれを取るか）を条件に書く」を足す案。

使ったスキル: `implement-issue`、`ja-text-communication`

## 実装者メモ（round 2）

対象 head: 970fd62f1989b9db719f0cdecf4bb2f0e9291807 のレビュー指摘への対応。

1. レビュー指摘と対応:
   - must-fix「`mise run verify` が exit 0 でない」: sandbox 内で `/tmp` の所有者が `nobody`（uid 65534）に見えるため `verify-run.test.sh` の `dir_chain_safe` が落ちる環境起因は Issue #692 のとおり。一方、sandbox 内の `/` は uid 1000 所有の書き込み可 tmpfs なので、`TMPDIR=/verify-tmp`（自分所有・祖先すべて自分所有）を指定して再実行したところ exit 0 で通った（2026-11-24 19:44 JST に終了、182 秒）。`verify-run.test.sh` 自身も「TMPDIR をその設定の届かない場所にしてやり直す」運用を想定しており、テストの性質上 TMPDIR の指定は検査を弱めない。
   - must-fix「調査文書に時系列の記述」: `docs/research/docker-sandboxes.md` から確認日時・「直近のものは修正が入っている」等の時系列表現を取り除き、「minisforum で確認」の根拠コマンドと日時は本 worklog の節（下記）へ移した。文書側は `docs/worklogs/issue-689.md` へのポインタを残す。
2. 対応での判断: `TMPDIR` に `/dev/shm` を先に試したが mise が一時ファイル作成で ENOENT を返した（`/dev/shm` は小さな tmpfs で mise の使い方と合わない模様）。`/` 直下に `verify-tmp` を作って代用した。sandbox 側で新しく書き込める場所に TMPDIR を向ける対処は Issue #692 の恒久対応で整理する。
3. 変更ファイル: `docs/research/docker-sandboxes.md`（時系列の除去）、本ファイル（確認コマンドの日時・verify 再実行の記録）。

## `mise run verify` の再実行記録

```text
$ mkdir -p /verify-tmp && chmod 700 /verify-tmp
$ TMPDIR=/verify-tmp mise run verify
（全タスク成功、script-test 含む）exit=0
```

- 2026-11-24 19:44 JST に終了、実装スレッド（agent-sandbox 内）で実行。`/verify-tmp` は `/`（uid 1000 所有の書き込み可 tmpfs）直下に作った自分所有ディレクトリ。`/tmp`・`/var/tmp`・`/run/shm` は sandbox 内では `nobody` 所有または read-only で `dir_chain_safe` の祖先検査を通らない。
- 初回の失敗（`/tmp` 起因）と本対処の記録は実装者メモ（round 1・2）と Issue #692 を参照。

## 受け入れ条件「導入・設定変更・サインインを何もしていない」の確認

2026-11-24 19:07 JST、実装スレッド（agent-sandbox 内）で実行。

```text
$ command -v sbx
（出力なし）
exit=1
$ id -nG
hatai nogroup
exit=0
```

- `command -v sbx` が何も返さず、`id -nG` に `kvm` が無い。導入・グループ変更・サインインは行っていない。
- 補足: 実装担当の sandbox の内側のため、`/dev/kvm` はこの名前空間では見えない（ホスト側の記録では `root:kvm`・`crw-rw----` で存在。Issue #689 本文、2026-11-24 18:57 JST）。

## minisforum で確認：調査文書の根拠コマンドと日時

調査文書の「minisforum で確認」の項目は、以下のコマンドで確かめた。初回は 2026-11-24 19:05 JST 頃、時系列を文書から worklog へ移す見直し（round 2）に合わせて 19:34 JST に再実行し、いずれも同じ結果。

```text
$ uname -a
Linux minisforum 6.18.58.1-microsoft-standard-WSL2 #1 SMP PREEMPT_DYNAMIC Tue Oct 27 00:24:11 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux
$ lsb_release -ds
Ubuntu 24.04.5 LTS
$ lsmod | grep -E '^kvm'
kvm_amd                   -2  -2
kvm                       -2  -2
$ id -nG
hatai nogroup
$ command -v sbx
（出力なし）exit=1
$ stat -c '%u %a' /tmp
65534 1777
```

- `/dev/kvm` は実装担当の sandbox（bwrap）の内側からは見えない（sandbox の `/dev` は最小構成）。ホスト側で `root:kvm`・`crw-rw----` で存在する記録は Issue #689 本文（2026-11-24 18:57 JST）。
