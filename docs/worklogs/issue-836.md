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

- 2026-11-26 20:44:43 JST、21:43:43 JST、21:56:44 JST、23:18:07 JST と 2026-11-27 00:09:12 JST。いずれも上と同じ結果。
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

## 実装者メモ（round 2）

対象 head: 942f0b31b7ffcffed95d04567f6d9a2105d08736 のレビュー指摘への対応。

1. 詰まった箇所: レビュー指摘と対応。must-fix 5 件のうち 4 件は `docs/research/microsandbox.md` の 4（試す段階の計画）にあり、本番の fnox 設定と鍵を VM に渡す案、既存の共有 git dir を書き込み可で渡す案、`msb self uninstall` の後に `msb df` を実行する順序、初回起動のイメージ取得を許可に含めていない点だった。試行専用の保存先・試験専用のクローン・試験専用の資格情報に分け、`config`・`config.worktree`・`hooks/` への書き込み拒否をエージェントの起動より先に確かめる順序に直した。残る 1 件は Issue へのコメントで、必要な許可と実機試用を保留した理由を Issue #836 に残した。nit のうち `[文書のずれ]` の 1 件（bwrap の書き込み先に `/tmp` がある）は 2 の表に反映した。印の無い nit のうち `[issue-1701]` の出典 URL だけは、受け入れ条件の「microsandbox の欄に出典 URL が付いている」に関わるため出典一覧に足した。ほかの印の無い nit（結論の範囲、この worklog のパスの一般化）は直していない。この round の `mise run verify` は `TMPDIR=/verify-tmp` で exit 0（2026-11-27 00:09:00 JST に終了、307.85 秒、`script-test: 59 tests ok`）。差分は文書だけで環境が変わらないため、素の実行は繰り返していない。
2. 曖昧だった文: 「試験専用のクローンで共有 Git ディレクトリを独立させ、現行の bwrap が保護する `config`、`config.worktree`、`hooks/` の書き込み拒否を、エージェントの起動より先に確認してください。」（PR #842 の codex のレビュー、must-fix）
3. 仮定と解釈: 試験専用のクローンの中で worktree を作り、Pelway と同じ構成（`.git` ポインタと共有 git dir）で確かめると解釈した。`config.worktree` は worktree にだけできるため。採らなかった解釈: worktree を作らない単純な clone で確かめる。「試験専用の資格情報」は `PELWAY_AGENT_FNOX_CONFIG` で指す試験用の fnox 設定と鍵と解釈した（`scripts/agent-gh` がこの環境変数で設定ファイルを切り替えるため）。
4. 足りなかった情報・検証手段: `msb self uninstall --yes` が保存先全体を削除することは、文書（`docs/cli/management.mdx`）には「msb・libkrunfw・コマンドのリンク」としか書かれておらず、ソース（`crates/cli/lib/commands/self_cmd.rs` の `uninstall_all`）からしか分からなかった。`MSB_HOME` で試行専用にした保存先を `msb self uninstall` が対象にするかは、round 3 のレビュー指摘を受けてソースで確かめた: `resolve_base_dir()` は `resolve_home()` を呼び、空でない `MSB_HOME` をそのまま使い、未設定なら `~/.microsandbox` になる（`crates/cli/lib/commands/self_cmd.rs`・`crates/utils/lib/lib.rs` の `resolve_home()`）。削除対象は実行ファイルの場所ではなく実行時の `MSB_HOME` で決まる。
5. 改善案: `docs/research/docker-sandboxes.md` の「4. 実際に試す段階の計画」にも同じ指摘が当たりうる（本番の `~/.config/pelway-agent` をマウントする案 (b)、既存の worktree での検証 (b)）。`sbx` を試す Issue を立てるときに見直す。`.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に、実機で試す計画を書く Issue では本番の環境と資格情報から分けることを受け入れ条件に含める例を足す。

使ったスキル: `implement-issue`、`write-pull-request`、`ja-text-communication`

## 実装者メモ（round 3）

対象 head: 6be1fb531e8ee5ec3b59cf789d2fa7bc80e1e067

1. 詰まった箇所: レビュー指摘と対応（codex の must-fix 3 件と claude の must-fix 1 件。いずれも `docs/research/microsandbox.md` の 4 に集中）。codex の指摘: 導入手順が `install.sh` を使う前提で、既存の `~/.local/bin/msb`・`~/.local/bin/microsandbox` のリンクを書き替えてしまう点（手順 1 を GitHub Releases からの手動導入に置き替え、試行専用ディレクトリー `<MSB_TRIAL>` を導入）。後片付けが `msb self uninstall --yes` 依存で、コマンドリンクの削除と旧形式のシェル設定の清掃まで行う点（`msb ps -a`・`msb rm --force`・`msb df --verbose` で個別に消し、`<MSB_TRIAL>` を `rm -rf` する手順に置き替え）。確認 (c) が `.git/config` と `.git/hooks/` の拒否だけで、worktree ごとの `config.worktree`（`.git/worktrees/<id>/config.worktree`）を抜いていた点（`git config extensions.worktreeConfig true` の設定と `git rev-parse --git-path config.worktree` で位置を確認する手順を追加）。claude の指摘: 「試験専用の資格情報」が置き場所だけで決まっていて、pelway-dev の秘密鍵を別の age 鍵で暗号化した試験用の fnox 設定でも満たせてしまう点（許可表 #4 と検証 (e) で、試験用のリポジトリにだけインストールした別の GitHub App の鍵に限定し、pelway-dev・pelway-review の鍵は試験用設定にも入れないと明記。モデルのキーは試行のために発行して失効させるものに限定）。(f) が T3 の接続の資格情報を VM に渡す検証で、本番の T3 の acpRegistry を変える点（(f) を資格情報を渡さない・本番の T3 を変えない形に限定し、渡す形は別の許可が要ると明記）。nit のうち `[文書のずれ]` の 2 件（行 44 の出典を `[volumes]` から `[sec-fs]` に、README 27 行の判定一覧を v0.7 選定の文書に限定）と worklog の実行記録の欠落（23:18:07・00:09:12 の分を追記）は直した。`MSB_HOME` を全呼び出しで前置する nit は手順 1 の置き替えで吸収した。この round の `mise run verify` は `TMPDIR=/verify-tmp` で exit 0（2026-11-27 10:15 JST に終了、176.18 秒、`script-test: 59 tests ok`）。round 2 と同じく `/tmp` の所有者が uid 65534 に見える環境起因（Issue #692）のため TMPDIR を分けた。差分は文書だけで、素の実行は繰り返していない。
2. 曖昧だった文: 「削除の対象は install の場所ではなく `MSB_HOME`（既定 `~/.microsandbox`）であることをまず明記し、`self uninstall` ではなく、保存先の個別ディレクトリと `~/.microsandbox` の残りを消す手順に置き替えてください。」（PR #842 の codex のレビュー、must-fix）「個別ディレクトリ」と「`~/.microsandbox` の残り」の対応関係が分かりにくいが、削除対象は実行時の `MSB_HOME` で決まるため、試行専用に分けた `<MSB_TRIAL>` ごと `rm -rf` する形と解釈した。`msb df` の非 verbose 表示が種類ごとの集計で個別パスを出さない点も指摘されており、`--verbose` の記述を足した。
3. 仮定と解釈: `<MSB_TRIAL>` は試行のためだけの保存先で、導入した実行ファイルとランタイムデータを同じ木に置くと解釈した。インストーラーが `~/.local/bin` に置くコマンドリンクは試行では作らないため、後片付けでリンクの削除もシェル設定の清掃も要らない。`config.worktree` の拒否確認は、worktree 機能（`extensions.worktreeConfig`）を試験専用のクローンで有効にした上で行うと解釈した（worktree を作らないと `config.worktree` のファイルが生成されないため）。「試験専用の資格情報」は、試験用のリポジトリにだけインストールした別の GitHub App の秘密鍵を `PELWAY_AGENT_FNOX_CONFIG` で指す試験用の fnox 設定に入れたものと解釈した（レビューの「別の GitHub App の鍵で、pelway-dev・pelway-review の鍵は使わない」に従う）。採らなかった解釈: pelway-dev の鍵を別の age 鍵で暗号化し直した設定。GitHub App の秘密鍵はその App の全インストール先のトークンを発行できるため、試験用のリポジトリを pelway-dev のインストール先に足すだけで本番への書き込みが可能になり、試験専用とは呼べない。(f) は「T3 の接続の資格情報を渡さない形に限る」方を選んだ（もう一方の選択肢「使う T3 と資格情報の受け渡しを許可の一覧に足す」は、本番の T3 の acpRegistry を変える検証を許可に含めることになり、「本番の T3 スレッドでは使わない」と矛盾が残るため）。
4. 足りなかった情報・検証手段: `install.sh` の挙動（リンクの上書き、`MSB_HOME` で変えられるのは保存先のみ）と `msb self uninstall` の挙動（`clean_legacy_shell_config` による `~/.profile`・`.bashrc`・`.zshrc`・fish conf の清掃、`resolve_home()` による削除対象の決定）は文書になく、`scripts/install.sh` と `crates/cli/lib/commands/self_cmd.rs`・`crates/utils/lib/lib.rs` のソースからしか分からなかった。`msb df` の `--verbose` が個別パスを出すことは `crates/cli/lib/commands/storage.rs` から確かめた。worktree ごとの `config.worktree` は git v2.51.0 の `git-worktree.adoc`（`extensions.worktreeConfig` と `worktrees/<id>/config.worktree`）で位置を確かめた。GitHub App の鍵が全インストール先のトークンを発行できることは `scripts/agent-gh` と `scripts/setup/agent-accounts.sh`（dev・review・notify を別の App・別の age 鍵で分けている構成）から確かめた。
5. 改善案: 「サードパーティーのツールの導入・削除を文書に書くとき、インストーラーや uninstall コマンドの副作用（コマンドリンク・シェル設定・既存ファイルの上書き）を先にソースで確かめる」ことと、「試験専用の資格情報を要求するときは置き場所ではなく中身（どの App・どの鍵・失効させるか）で定める」ことを `.agents/skills/write-issue/SKILL.md` またはこの調査型 Issue の例に書く。現行の docker-sandboxes の「試す段階の計画」も `sbx` コマンドの削除系の副作用と試験用の資格情報の中身を確かめておくと、この指摘の形を防げる。

使ったスキル: `implement-issue`、`ja-text-communication`
