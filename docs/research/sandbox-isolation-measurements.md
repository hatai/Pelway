# サンドボックス測定:gVisor(runsc)と crun の隔離比較

## 結論

測定環境(WSL2 上の rootless Podman)では、gVisor は動きましたが、3 つの追加設定(cgroup 無視、プラットフォーム変更、ランタイム引数の受け渡し方)が要りました。起動時間は +0.13 秒(約2割増)、常駐メモリはサンドボックスごとに約50MB増、egress 拒否(`--network=none`)は crun と同じく保持されました。libkrun(crun+krun)は、WSL2 カーネルに /dev/kvm があるにもかかわらず、実行ユーザーが kvm グループに属していないため rootless からは使えず、測定を断念しました。

## この文書の前提と読み方

- 測定日は 2026-11-12 です。Issue #94 の受け入れに沿って、rootless Podman との組み合わせ、起動時間、syscall の互換性、資源の使用量、egress 拒否、WSL2 での KVM 可否を測りました。
- 測定環境は次のとおりです。
  - ホスト: Windows 上の WSL2(カーネル `6.18.58.1-microsoft-standard-WSL2`、Ubuntu 24.04、24 CPU、RAM 40.9 GB)
  - Podman 4.9.3(rootless、ネットワークは netavark+pasta、cgroup は systemd を `--ignore-cgroups` の間のみ cgroupfs に変更)
  - crun 1.14.1(比較の基準)
  - gVisor runsc `release-20261109.0`(公式リリースの `gvisor-x86_64.tar.zstd`、SHA256SUMS で検証)
- 測定はすべて hatai ユーザー(rootless)で実行しました。コンテナイメージは `docker.io/library/postgres:16-alpine` と `quay.io/podman/hello` です。
- 判定は「事実」と「解釈」に分けて書きます。事実はそのままの測定結果です。

## 1. rootless Podman と組み合わせて動くか

### gVisor:動く。ただし3つの設定が要る

事実:

1. runsc を素のまま podman のランタイムに指定すると、runsc の cgroup セットアップが systemd D-Bus の polkit 認証に拒否されて起動しません(`systemd error: Interactive authentication required`)。
2. podman 側の cgroup manager を cgroupfs に変えても、runsc が `/sys/fs/cgroup/cgroup.subtree_control` を開いて失敗します(permission denied)。
3. runsc に `--ignore-cgroups` を渡すとこの失敗を回避できます。podman 4.9.3 は `containers.conf` の `[engine.runtimes]` に配列で引数を書いても conmon に渡さないため、runsc を 1 枚のシェルスクリプトで包んで `--ignore-cgroups` を常時付ける必要がありました。
4. cgroup を無視しても、既定の systrap プラットフォームは起動に失敗します。runsc のログには `failed to install seccomp notify rules: device or resource busy` と出て、コンテナの作成が "out of memory" で止まります。gVisor の GitHub issue には、この環境特有の直接の一致は見つかりませんでした(2026-11-12 時点)。
5. `--platform=ptrace` に変えると起動します。ptrace は systrap より性能が劣ることが公式に説明されています。
6. `--platform=kvm` は /dev/kvm の権限問題(後述)により rootless では使えません。

解釈:WSL2 カーネルと systrap の組み合わせが当環境では壊れています。ptrace で動かせはしますが、gVisor の性能の目安を測るには、本来 systrap(または KVM)での測定が必要です。

### crun+krun(libkrun):rootless では動かせない

事実:

1. WSL2 カーネルは KVM を持っています(`/dev/kvm` が `root:kvm` 660 で存在、`/proc/misc` に 232=kvm)。
2. 実行ユーザー hatai は kvm グループに属していません(`groups` に kvm がなく、`test -r /dev/kvm` が失敗)。rootless プロセスから /dev/kvm を開けません。
3. Ubuntu 24.04 の crun 1.14.1 は krun サポートなしのビルドです(`crun --version` の機能一覧に krun がなく、podman のデバッグログも「no valid executable found for OCI runtime krun」と報告します)。
4. sudo はパスワード必須のため、測定から kvm グループへの追加や /dev/kvm の権限変更は行いませんでした。

解釈:WSL2 カーネル自体は KVM を提供しますが、rootless Podman で libkrun を使うには、実行ユーザーを kvm グループに加える管理操作が別に要ります。この操作は人間の判断で行うべきと考え、本測定では「KVM は存在するが rootless からは開けない」までを事実として残します。libkrun の起動時間や互換性の測定は、権限が整った後に別途行う必要があります。

## 2. 起動時間・資源の使用量

事実(`podman run --rm <イメージ> true` の wall time、7 回の中央値):

| ランタイム | 中央値 | 最小 | 最大 |
|---|---|---|---|
| crun 1.14.1 | 0.641 秒 | 0.617 秒 | 0.834 秒 |
| runsc(ptrace) | 0.770 秒 | 0.642 秒 | 0.988 秒 |

事実(sleep 常駐中のプロセス RSS、`ps` 実測):

| ランタイム | 常駐プロセスと RSS |
|---|---|
| crun | conmon 2.6 MB(ランタイムは起動後 exit、非常駐) |
| runsc(ptrace) | gvisor_sentry 35.2 MB + runsc-gofer 12.3 MB + conmon 2.3 MB(計約 50 MB/サンドボックス) |

- runsc のサンドボックスは 261 スレッド(ptrace のタスク分)を持ちます。
- `--ignore-cgroups` のため、podman stats の CPU% やメモリ値は cgroup 統計が取れず信頼できません(152031% などの異常値)。資源の実測はプロセス単位の `ps` で行いました。

解釈:タスク使い捨てコンテナの起動が約 0.13 秒(2割)遅れ、サンドボックスごとに約 50 MB の追加常駐が乗るペナルティは、Pelway の用途(1 task = 1 コンテナ)では許容範囲と考えられます。ただし systrap が使えない環境での値であり、ptrace は gVisor の中でも遅い部類である点に注意します。

## 3. syscall の互換性

事実(postgres:16-alpine の busybox 一式、22 コマンドで確認):

- crun・runsc(ptrace)とも、uname、/proc/cpuinfo、ps、mount、dd+sha256sum、tar、gzip などを含む 22 コマンドすべてが成功しました。
- カーネルの見え方の差:

| 項目 | crun | runsc |
|---|---|---|
| `uname -a` | ホストと同じ `6.18.58.1-microsoft-standard-WSL2` | 合成の `4.19.0-gvisor` |
| `/proc/kallsyms` | あり | なし |
| `dmesg` | ホストのカーネル起動ログ(WSL のコマンドライン引数を含む) | gVisor の合成ログ |
| `/proc/self/status` の Seccomp | 2(podman のプロファイル) | 0(sentry が syscall を代理) |
| `/proc/meminfo` の MemFree | 実際の空きに連動 | 実態と無関係に「ほぼ全量空き」 |

解釈:gVisor の procfs は合成のため、ホストの情報を容器から読み取れません(crun は dmesg でホストのカーネルコマンドラインが見えます)。反面、MemFree などエミュレーション値を前提にしたプログラムは誤動作の余地があります。Pelway の sandbox で動かす想定のワークロード(Rust、tokio の epoll/futex ベース)は gVisor の対応範囲にあります。io_uring は gVisor が未対応の既知の制限で、利用するライブラリの選定時に注意します。

## 4. egress deny を保てるか

事実:

| 測定 | crun | runsc(ptrace) |
|---|---|---|
| `--network=none` での DNS | 失敗(拒否される) | 失敗(拒否される) |
| `--network=none` での HTTP | 失敗(拒否される) | 失敗(拒否される) |
| 既定ネットワークでの HTTP | 成功(対照) | 成功(対照) |

- runsc のコンテナ内には `tap0`(10.0.2.100/24、gateway 10.0.2.2、DNS 10.0.2.3)が見え、podman の rootless ネットワーク(pasta)と接続しました。これは「Podman がユーザー名前空間を用意する方式で、ネットワーク名前空間と Netstack が使えるか」という oss-agent-execution-and-safety.md の未確認事項への実測回答です:使えます。

解釈:egress deny はランタイムではなく podman 側のネットワーク設定で決まり、gVisor でも `--network=none` はそのまま効きます。既定ネットワークではどちらのランタイムも外に出られるため、Pelway の「sandbox は gateway 以外拒否」を実現するには、runsc でも crun でも明示的な拒否設定が必要です。gateway 宛てのみ許可する部分許可の検証は、A6 の深掘りとして別 Issue に残します。

## 5. WSL2 の中で KVM が使えるか(libkrun)

事実:前述のとおり、WSL2 カーネルは /dev/kvm を持ちますが、rootless ユーザーは kvm グループ非所属のため開けません。sudo によるグループ追加・権限変更が必要です。

解釈:WSL2 で libkrun を使う道は「カーネルは対応済み、権限設定が残っている」状態です。Windows 側の nested virtualization が有効になっていること(=/dev/kvm が現れること)は、この WSL2 環境ではすでに満たされています。

## 6. A6 への示唆(筆者の整理)

- gVisor は rootless Podman で機能し、egress 拒否も podman 側で保持できます。kernel 共有をやめる隔離強化として、crun と同じ運用枠で試せます。
- ただし WSL2 環境では systrap が使えず ptrace での実行になります。systrap が壊れる原因の追跡と、KVM プラットフォームの権限問題が解けない限り、gVisor を WSL2 の既定にする判断はできません。
- libkrun は kvm グループ参加という管理操作が前提になります。管理者権限なしで運用したい Pelway の条件(A6 の確認欄)と組み合わせると、WSL2 では gVisor、Linux ホストでは libkrunという使い分けが現実的な候補になります。
- 起動 +0.13 秒・常駐 +50 MB は、1 task 1 コンテナの使い方では受け入れられる差です。

## 出典

- [gVisor rootless, 2026/11] gVisor. "Rootless." https://gvisor.dev/docs/user_guide/rootless/
- [gVisor platforms, 2026/11] gVisor. "Changing Platforms." https://github.com/google/gvisor/blob/release-20261109.0/g3doc/user_guide/platforms.md
- [gVisor runsc config, 2026/11] google/gvisor. runsc/config/config.go(--ignore-cgroups の定義)。https://github.com/google/gvisor/blob/release-20261109.0/runsc/config/config.go
- [crun 1.14.1] Ubuntu 24.04(noble)の crun パッケージ。krun なしビルド。
- [Pelway 08 (2026/11), A6] 本リポジトリ `docs/technical/v0.7/body/08-decisions-open.md`
- [oss-agent-execution-and-safety, 2026/11] 本リポジトリ `docs/research/oss-agent-execution-and-safety.md`

## 附録: 再現手順

runsc の取得(`scripts/agent-gh dev` は GitHub App の名義で gh を呼ぶラッパー):

```bash
scripts/agent-gh dev release download release-20261109.0 --repo google/gvisor \
  --pattern 'gvisor-x86_64.tar.zstd' --pattern 'SHA256SUMS' --dir <dir>
cd <dir> && grep 'gvisor-x86_64.tar.zstd' SHA256SUMS | sha256sum -c -
tar --zstd -xf gvisor-x86_64.tar.zstd   # runsc と gvisor-bin/ が展開される
```

podman 側の準備。podman 4.9.3 は `[engine.runtimes]` に書いた引数を conmon に渡さないため、runsc を包むラッパーでフラグを固定します:

```sh
#!/bin/sh
# cgroup 委任が無い rootless 環境向け。systrap が seccomp notify で EBUSY になるため ptrace。
exec <dir>/runsc --ignore-cgroups --platform=ptrace "$@"
```

実行と確認:

```bash
podman --runtime <ラッパー> run --rm docker.io/library/postgres:16-alpine uname -a
# → Linux ... 4.19.0-gvisor ... (crun だとホストと同じ 6.18.58.1-microsoft-standard-WSL2)

# 起動時間: podman run --rm <イメージ> true の wall time を 7 回取り中央値
# 資源: sleep 常駐中に ps -eo pid,rss,comm で runsc-gofer / gvisor_sentry / conmon を測る
# egress: --network=none と既定ネットワークで DNS(nslookup/getent)と HTTP(wget)の成否
```

systrap の失敗の再現(ラッパーから `--platform=ptrace` を外して実行)。runsc の `--debug --debug-log=<dir>/` で取ったログに `failed to install seccomp notify rules: device or resource busy` が出ます。
