# Issue #836 の作業ログ

## 実装者メモ（round 1）

対象 head: b99f0411736411fd72530241b4550dd2e6c5b3a6

1. 詰まった箇所: 手順 6: 素の `git commit` でコミットを作ってしまい、`scripts/agent-gh dev git push` に「pelway-dev[bot] 以外の名義のコミット」と止められた。push 前だったので `git reset --soft HEAD~1` で戻し、`scripts/agent-gh dev git commit` で作り直した。手順 6: `mise run verify` が `.agents/skills/verify-pelway/verify-run.test.sh` の 6 件で失敗した（下の「`mise run verify` の記録」。Issue #692 と同じ環境起因）。調査: 1 回目の取得で、ソース候補の抽出条件を `crates/*/src/` にしたところ、microsandbox の crate は `lib/` 配置で CLI のソースが取れなかった。2 回目で取り直した。調査: GitHub の検索 API で `repo:containers/libkrun ...` が HTTP 422 だった（リポジトリーは `libkrun/libkrun` に移っている）。新しい名前で検索し直した。
2. 曖昧だった文: 「`docs/research/README.md` に新しい文書の行がある（ほかの文書の載せ方に合わせる。載せる決まりが無ければ不要）」（Issue #836 の受け入れ条件）。`docker-sandboxes.md` は README の表に載っていない。一方、skill `write-issue` の手順 1 には「結果は `docs/research/<slug>.md` に出典付きで書き、`docs/research/README.md` の表に 1 行足す」とある。
3. 仮定と解釈: README の行は足した（載せる決まりがあるため）。`coding-agent-concurrency-config.md` の行と同じく「開発環境の道具の調査で、v0.7 の技術選定には含めない」と注記した。`docker-sandboxes.md` の行は足していない（Issue が求めていないため）。「minisforum で確認（コマンドと日時 JST）」の日時は、調査文書に書かずこの worklog に置いた（PR #693 のレビューで時系列の記述を指摘されたため）。ネットワークは `scripts/agent-gh dev` 経由の GitHub だけを使った（skill `implement-issue` の決まり）。そのため docs サイト（`https://docs.microsandbox.dev/`）と `https://microsandbox.dev/` は取得していない。公式リポジトリーの `docs/` を出典にして、URL は固定 commit のファイルを指した。採らなかった解釈: docs サイトを Web で取得して確認する（Issue の関連情報に URL があるが、上の決まりに反するため）。
4. 足りなかった情報・検証手段: bind mount 上の ELF の `execve` の回帰（microsandbox の Issue #2121）が v0.8.3 で残るか。公開 Issue と修正 PR が未解決であることと、v0.8.1〜v0.8.3 のリリースノートに修正の記載が無いことしか分からなかった（起動しないと確かめられない）。microsandbox の `stat-virt`・`host-perms` の既定値の明文（文書には選択肢の列挙だけで、Issue #2063 の報告から既定を推定した）。`mise run verify` を素のままで通す手段（`/tmp` の所有者が sandbox 内で uid 65534 に見える）。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり」に「GitHub 外の公式文書を取得してよいか。読めないときはリポジトリ内の同源の文書を固定 commit で出典にすること」を足す。`.agents/skills/write-issue/SKILL.md` の手順 1 に「`docs/research/README.md` に載せる対象（v0.7 の技術選定以外の、開発環境の道具の調査を含むか）」を明記する。

使ったスキル: `implement-issue`、`write-pull-request`、`ja-text-communication`、`.agents/skills/research`（背景エージェントは立てず、同じ方針（一次情報・出典付き）で実装者が調査した）

## 受け入れ条件「導入・設定変更・サインインを何もしていない」の確認

実装スレッド（agent-sandbox 内）で 3 回実行した。いずれも同じ結果。

```text
$ command -v msb
（出力なし）
exit=1
$ command -v microsandbox
（出力なし）
exit=1
$ id -nG
hatai nogroup
exit=0
```

- 2026-11-26 20:44:43 JST、21:43:43 JST、21:56:44 JST。
- 21:43:43 JST の追加分: `test -e ~/.microsandbox` は exit=1。`ls -l ~/.local/bin/msb ~/.local/bin/microsandbox` は両方とも `No such file or directory`。`getent group kvm` は `kvm:x:993:`（所属者なし）。
- microsandbox・Docker・Kiri・WSL・Windows 機能の導入と設定変更、sudo、再起動、外部サービスへのサインインはしていない。GitHub は `scripts/agent-gh dev api` と `scripts/agent-gh dev git`（fetch・commit・push）だけを使った。公式リポジトリは clone せず、GitHub API でファイルを読んだ。

## minisforum で確認：調査文書の根拠コマンドと日時

`docs/research/microsandbox.md` の「minisforum で確認」の項目は、次のコマンドで確かめた。実装スレッド（agent-sandbox 内）で実行した。

```text
$ uname -a
Linux minisforum 6.18.58.1-microsoft-standard-WSL2 #1 SMP PREEMPT_DYNAMIC Tue Oct 27 00:24:11 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux
$ lsb_release -ds
Ubuntu 24.04.5 LTS
$ getconf GNU_LIBC_VERSION
glibc 2.39
$ lsmod | grep -E '^kvm'
kvm_amd                   -2  -2
kvm                       -2  -2
$ grep -w kvm /proc/misc
232 kvm
$ grep -m1 '^flags' /proc/cpuinfo
（flags に hypervisor・svm・avx2・bmi1・bmi2・movbe を含む）
$ getent group kvm
kvm:x:993:
$ id -nG
hatai nogroup
$ ls -l /dev/kvm
ls: cannot access '/dev/kvm': No such file or directory
$ git rev-parse --show-toplevel --git-dir --git-common-dir
/home/hatai/.t3/worktrees/Pelway/agent-836-microsandbox--cad860c4594c4ee2
/home/hatai/repos/hatai/Pelway/.git/worktrees/agent-836-microsandbox--cad860c4594c4ee2
/home/hatai/repos/hatai/Pelway/.git
$ git worktree list | wc -l
263
$ find <worktree> -path <worktree>/target -prune -o -type l -print | wc -l
108
```

- 2026-11-26 20:44:43 JST に `uname`・`lsb_release`・`lsmod`・`id`・`id -nG`・`command -v`・`ls -l /dev/kvm`・`/proc/misc`、`grep -c -w -E 'svm|vmx' /proc/cpuinfo`（結果は 24）を実行した。`lsmod` はこの 1 回だけである。21:43:43 JST に `uname`・`lsb_release`・`getconf GNU_LIBC_VERSION`・`stat -f -c '%T' /home/hatai /tmp`（両方とも `ext2/ext3`）・`nproc`（24）・`free -g`（合計 38 GiB）・`/proc/cpuinfo` の flags（`hypervisor`・`svm`）・git のパスを実行した。21:56:44 JST に `umask`（0022）・祖先ディレクトリの所有と mode・`du -sh /home/hatai/repos/hatai/Pelway/.git`（178M）・`du -sh --exclude=target <worktree>`（21M）・worktree の通常ファイルの数（1571）・symlink の数（108）を実行した。
- `lsmod` は stderr に libkmod の「could not open '/sys/module/.../holders'」行を出す（sandbox 内で `/sys` が見えないため）。stdout の 2 行は #689 の記録と同じ。
- `/dev/kvm` は実装担当の sandbox の内側からは見えない（`/dev` が最小構成）。ホスト側で `root:kvm`・`crw-rw----` で存在する記録は `docs/research/docker-sandboxes.md`。
- `/proc/cpuinfo` の flags のうち `avx2`・`bmi1`・`bmi2`・`movbe` は、read ツールと grep ツールで `/proc/cpuinfo` を直接読んで確かめた（2026-11-26 JST。シェルを使っていないため時刻は記録していない）。
- `/home` は sandbox 内では `nobody:nogroup` に見える（`stat -c '%U:%G'`）。理由は確認していない。`/home/hatai` は `750 hatai:hatai`。

## GitHub からの取得

- `scripts/agent-gh dev api` で `superradcompany/microsandbox`（commit `0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff`）の文書・ソース・公開 Issue・PR・リリース・セキュリティ勧告と、`superradcompany/libkrunfw`（commit `f0493b6f1873ed6031bc0bb87568db0095a6d3b6`）を読んだ。公開 Issue と PR 49 件（うち PR 3 件）の本文とコメント、PR 5 件の merge 状態、リリース 4 件（v0.8.3・v0.8.2・v0.8.1・v0.7.16）、セキュリティ勧告 14 件を取得した。
- 取得スクリプトと生データは作業用の一時領域（`/tmp/opencode/issue-836/`）に置いた。リポジトリにはコミットしていない。

## `mise run verify` の記録

実装スレッド（agent-sandbox 内）で実行した。

```text
$ stat -c '%u %a %n' / /tmp
1000 755 /
65534 1777 /tmp
$ mise run verify
（.agents/skills/verify-pelway/verify-run.test.sh の 6 件が失敗）exit=1
$ mkdir -p /verify-tmp && chmod 700 /verify-tmp
$ TMPDIR=/verify-tmp mise run verify
（全タスク成功、script-test: 59 tests ok）exit=0
$ mise run pr-size
pr-size: +179 / -0（追加の上限 800、削除の上限 2000）
```

- 素の `mise run verify` は 2026-11-26 22:50:31 JST に終了（269.27 秒）。失敗は `make_run_dir` の 4 件と `locks_dir_ready` の 2 件で、`/tmp` の所有者が uid 65534 に見えることが原因（Issue #692）。
- `TMPDIR=/verify-tmp` での再実行は 2026-11-26 22:54:11 JST に終了（202.13 秒）。`/verify-tmp` は `/`（uid 1000 所有の書き込み可 tmpfs）直下に作った自分所有のディレクトリ。`verify-run.test.sh` は自身のケース用に `mktemp -d` を使い、TMPDIR の場所を変えても検査ロジックは変わらない。
