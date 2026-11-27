# microsandbox を実装担当の sandbox に使えるかの調査

Issue #836。microsandbox は導入せず、公式リポジトリー（`superradcompany/microsandbox`。commit `0dad6d43`、v0.8.3 の版）の文書とソース、公開 Issue・PR、ホストの読むだけのコマンドで確認しました。「minisforum で確認」と書いた項目のコマンドと日時（JST）は `docs/worklogs/issue-836.md` に記録します。

## 結論

**条件付きで試す**ことを推します。試すのは、ホストと共有するフォルダーの公開中の不具合（2 の git の行）が修正されてからです。`sbx` より先に microsandbox を試すべきです。理由は 5 に書きます。

## この文書の前提と読み方

- microsandbox は、libkrun（KVM などで軽量な仮想マシンを起動するライブラリー）を土台に、ローカルでマイクロ VM（軽量な仮想マシン）を起動するランタイムです。コマンドは `msb` です。
- 「確認」は公式リポジトリーの文書かソース、公開 Issue・PR に記載があるという意味で、実機での動作確認ではありません。「未確認」は今回の資料で裏付けられなかった項目で、機能が無いという意味ではありません（`docs/research/docker-sandboxes.md` と同じ流儀です）。
- bwrap 方式の列は `scripts/agent-sandbox/sandbox-shell` と `docs/dev-environment.md` の「実装担当の sandbox（agent-sandbox）」の記述に基づきます。`sbx` の列は `docs/research/docker-sandboxes.md` から引き写しました（再調査はしていません）。
- 出典の URL はリポジトリーの固定した commit のファイルを指します。docs サイト（`https://docs.microsandbox.dev/`）は同じ `docs/` 配下を生成元にしていますが、サイトの画面はこの調査では確認していません。
- 公開 Issue・PR の「open」「未マージ」はこの文書を書いた時点の状態です。

## 1. 対応環境：WSL2 の minisforum で microsandbox が動く見込み

### 公式の条件と minisforum の状態の突き合わせ

| 公式の条件 | minisforum の状態 | 判定 |
|---|---|---|
| Linux は glibc 2.28 以降 [linux-trouble] | glibc 2.39（`getconf GNU_LIBC_VERSION`）。minisforum で確認 | 合う |
| KVM が有効で `/dev/kvm` がある [linux-trouble] | `lsmod` で `kvm_amd` と `kvm` が読み込み済み。`/proc/misc` に `kvm` がある。minisforum で確認 | 合う（下のグループを除く） |
| 利用者が `/dev/kvm` を読み書きできる。多くのディストリビューションは `kvm` グループで許可する。setuid も特別なケイパビリティーも要らない [linux-trouble] [isolation] | `id -nG` は `hatai nogroup` で `kvm` を含まない。`getent group kvm` は `kvm:x:993:`（所属者なし）。`/dev/kvm` は実装担当の sandbox の内側からは見えない（`/dev` が最小構成のため）。ホスト側の記録では `root:kvm`・`crw-rw----`（`docs/research/docker-sandboxes.md`）。minisforum で確認 | **未達。`kvm` グループへの追加が要る** |
| VM やコンテナーの中で動かすときは、外側が入れ子の仮想化を出していること [linux-trouble] | WSL2 は Hyper-V の上で動く VM。`/proc/cpuinfo` の flags に `hypervisor`（カーネルがハイパーバイザーの上で動いている印）と `svm`（AMD の仮想化拡張）の両方がある。minisforum で確認 | 追加の Windows 側の設定は不要の見込み |

ゲストカーネルについての報告もあります。v0.7.15 以降のゲストカーネルは x86-64-v3 の命令を含み、AVX2・BMI1・BMI2・MOVBE のいずれかが無い CPU では sandbox が起動しないというものです [issue-2127]。minisforum の `/proc/cpuinfo` の flags には `avx2`・`bmi1`・`bmi2`・`movbe` があります（minisforum で確認）。

### WSL2 は対応環境か

- 公式文書の Linux のページに WSL2 の記述はありません [linux-trouble]。入れ子の仮想化については「クラウドの VM・CI ランナー・別のハイパーバイザーの中で動かすなら、外側が入れ子の仮想化を出している必要がある」とあるだけです。
- 公開 Issue #920（closed）は WSL2 での報告です。1 回目は sandbox が起動し、2 回目からは `libkrun VM` が SIGABRT で落ちて、`/dev/kvm` の読み書きの検査も失敗に変わりました。メンテナー（appcypher）は「WSL は best-effort で、入れ子の仮想化が有効なら動くことはあるが、こちらでは試験していない」と回答しました。その後「Windows Hypervisor Platform（WHP）で動く正式な Windows 対応ができたので、WSL はもう優先度が高くない」としてクローズしました [issue-920]。
- 同じ Issue の別のコメント（一般ユーザー）による説明があります。WSL2 では udev の `uaccess` 規則が VM の作成のたびに `/dev/kvm` の ACL を書き換えるため、`setfacl` で足した許可は最初の VM の後で消える。`kvm` グループへの所属なら残る、という内容です [issue-920]。この説明は microsandbox ではなく Firecracker での再現に基づきます。
- Issue #47（closed）でも、メンテナーは「WSL2 はもう推奨しない。動くには動くが、WHP で動く正式版がある」と書いています [issue-47]。
- Windows ネイティブ版（WHP を使う）は「プレビュー」です [windows-trouble] [cli-overview]。`HypervisorPlatform` の Windows 機能が要ります。WSL2 のワークスペースを Windows 版に渡す経路は、この調査では採りません。Windows 機能の変更は Issue の止まる条件に当たり、WHP 固有の不具合の報告もあるためです（例: AMD Ryzen で VM が起動中に triple fault する報告 [issue-1095]）。

**見込みの結論**：minisforum は glibc と KVM の条件を満たし、残る操作は `kvm` グループへの追加だけです（`/dev/kvm` はこのスレッドからは見えず、現在のホストでの読み書き可否と、グループ追加で十分かは未確認です）。ただし upstream は WSL2 を試験対象外とし、優先度も下げています。minisforum の WSL2 カーネル（`6.18.58.1-microsoft-standard-WSL2`）で microVM が起動するかは、実際に起動するまで分かりません。

## 2. 今の bwrap 方式・Docker Sandboxes との比べ

| 観点 | bwrap 方式（現行） | `sbx` | microsandbox |
|---|---|---|---|
| ファイル | セッションの worktree・共有 git dir・ツールキャッシュに加えてホストの `/tmp` が書き込み可（`scripts/agent-sandbox/sandbox-shell`）。`/home`・`/etc` 等は read-only。秘匿パス（`~/.ssh`・`~/.gnupg`・`~/.docker`・`~/.config/gh`・`~/.config/opencode`・`~/.config/devin`・`~/.t3/userdata` 等）は tmpfs で隠す。worktree 内 symlink 越しの脱走は opencode プラグインの実体パス検査で防ぐ [docker-sandboxes] | microVM で、エージェントは VM 内のファイルシステムしか見えず、ホスト側は workspace として渡したパスだけが virtiofs 越しに見える。秘匿パスはマウントされないため到達不能。workspace 外への symlink は辿らない。直接マウントは workspace 全体が rw で、`.env` や未追跡ファイル・`.git/hooks` まで含む [docker-sandboxes] | ゲストからはイメージと、明示的にマウントしたものだけが見えます。ホストのパス・環境変数・資格情報は暗黙には渡りません [sec-fs]。ホストのディレクトリーは bind mount（`--mount-dir`）で virtio-fs 越しに共有します。`ro` はホスト側で強制されるため、ゲストの特権プロセスも書き込めません [sec-fs]。Linux 5.6 以降では `openat2` の `RESOLVE_BENEATH` で、`..` や symlink による共有ルートの外への脱出を防ぎます [sec-fs]。マウント内の一部のパスだけを隠す機能はなく、公開中の PR #1448 で提案段階です [pr-1448]。`~/.ssh` などの秘匿パスはマウントしなければゲストから到達できませんが、マウントした worktree 内の `.env` などのファイルは見えます |
| git | worktree の `.git` ポインタ先の共有 git dir（`git rev-parse --git-common-dir`）を rw で bind し、`config`・`config.worktree`・`hooks/` には read-only を重ねて url rewrite・hook 設置を防ぐ。push 後は `agent-gh-resign` が署名を作り直す流れが壊れない [docker-sandboxes] | **ホストの git worktree を直接マウントすると `.git` ポインタを解決できず、エージェントは git を使えない**。`--clone` も main ではない worktree からは拒否される。回避案（共有 git dir を同じ絶対パスに追加 workspace としてマウントする）は文書に記載が無く未確認。仮に通ったとしても、`config`・`hooks` に read-only を重ねる bwrap の保護を再現する手段が文書上見当たらない [docker-sandboxes] | bind mount した普通のリポジトリーで `git status`・`git diff` できる例が公式ガイドにあります [opencode-guide]。ホストと同じ絶対パスでマウントする使い方が公式の ACP ガイドにあり、入れ子のマウント先にも対応します [jetbrains-acp] [volumes]。`--mount-file` で 1 ファイルを `ro` でマウントできます [volumes]。Pelway で使うなら、worktree と共有 git dir（`/home/hatai/repos/hatai/Pelway/.git`。minisforum で確認）を同じ絶対パスでマウントする構成が候補です。共有 git dir 内の `config`・`config.worktree`・`hooks/` を `ro` で重ねられるかは**未確認**です。公開中の不具合が 4 件あります。(1) v0.7.16 から、bind mount 上の ELF バイナリーの `execve` が `EACCES` で失敗します（スクリプトは動きます）。修正 PR #2125 は未マージで、v0.8.1〜v0.8.3 のリリースノートに修正の記載はありません [issue-2121] [pr-2125] [release-v0.8.1] [release-v0.8.2] [release-v0.8.3]。(2) ゲストで作った symlink がホストでは通常のファイル（0600）になり、ホストの `git status` が typechange を出します。`stat-virt=off` にするとホストに本物の symlink ができますが、ゲストがホストの任意の場所を指す symlink を共有ディレクトリーに作れるようになります [issue-2063] [issue-743]。(3) ゲストで付けた実行ビットは、`host-perms=mirror` を選ばないとホストのファイルに反映されません [issue-742] [sec-fs]。(4) ホスト側で入れ替えたファイルがゲストから見えるまで数秒かかることがあります [issue-1551]。Pelway の worktree には 108 個の symlink があります（`.claude/skills/` 配下の `.agents/skills/` へのリンクを含みます。minisforum で確認）。エージェントは git 操作と `target/` へのビルドをするため、(1) と (2) は現行の運用に直接当たります。`agent-gh`・`agent-gh-resign` はリポジトリーのスクリプトなので、VM 内に `gh`・`fnox`・`jq` と資格情報があれば動く見込みですが**未確認**です。試すときは既存の共有 git dir を VM に渡しません。試験専用のクローンで保護を先に確かめてから、試験用のリポジトリと試験専用の資格情報で確かめます（4 の (c) と (e)） |
| ネットワーク | 遮断しない。ホストの経路をそのまま使う（#151 で hatai が「許可リストにしない」と指示）[docker-sandboxes] | 外向き TCP は全てホスト上のプロキシ経由でポリシー評価される。preset に Open（全 TCP 許可）があり、許可リストにしない方針とは TCP の範囲で両立する。UDP は experimental の egress を有効化しないと出られず、ICMP は常に遮断。DNS は内部リゾルバが担う [docker-sandboxes] | ゲストの通信はすべてホストプロセス内のユーザー空間ネットワークスタックを通り、ポリシーで検査されます [sec-net]。既定は「公開インターネットへの外向きは許可、私網（RFC 1918・キャリア級 NAT・IPv6 ULA）・ループバック・リンクローカル・クラウドのメタデータ・ホスト自身は拒否」です。受信は公開したポートだけに届き、既定は `127.0.0.1` に bind します [sec-net]。プロファイルは `public`・`private`・`host`・`all`・`none` です [net-overview]。DNS はホストが代理で解決し、既定ではホストの resolver リストを使います [dns]。ICMP は特権の不要な echo だけを通し、traceroute 系は代理しません [sec-net]。ホスト上のサービスは `host.microsandbox.internal` に届き、`host` を許可したときだけ使えます [net-overview]。ホストの Unix ソケットは `--vsock` で渡せます [host-sockets]。bind mount したソケット経由ではホストに届きません [issue-1290]。**許可リストにしない方針との両立**: 既定は許可リストではなく「私網を除いて許可」なので、公開サイトの調査はできます。bwrap より厳しい点は、ホスト自身と私網宛てが既定で拒否されることです。ホストのサービス（T3 の MCP サーバーなど）が要る場合は `host`・`private`・`all` を足します。ICMP の制約は bwrap にはありません |
| 資格情報 | `scripts/agent-gh` が呼び出しごとに fnox+age 鍵（`~/.config/pelway-agent/`、sandbox 内では read-only 可読）で GitHub App のトークンを発行し、`GH_TOKEN` を子にだけ渡す。モデル資格は devin の wrap 時のみ `~/.config/devin`・`~/.local/share/devin` を devin プロセスに可視にし、コマンドにはマスクする [docker-sandboxes]。opencode のプロセス自体はホストで動き、その設定（`~/.config/opencode`）はエージェントのコマンドから隠れる（`docs/dev-environment.md`） | ホスト側プロキシが宛先に応じて実資格を注入し、sandbox には sentinel（仮の値）しか見せない。`sbx secret set`（OS のキーチェーン。Secret Service が無い headless/WSL では `~/.config/com.docker.sandboxes` のファイルに退避）・`--command`（ホストでコマンドを実行して値を得る）・`--ref`・OAuth の経路がある。`github` サービスは `GH_TOKEN`・`GITHUB_TOKEN` を api.github.com・github.com 等への要求に注入する。モデル資格は agent ごとの service で注入する。`agent-gh` そのものは、VM 内に fnox と age 鍵が無いためそのままでは動かない。置き換え案は 2 つで、どちらも未確認 [docker-sandboxes] | secret にプレースホルダー（`$MSB_<環境変数名>`）を渡し、ホスト側のプロキシが TLS を復号して、許可したホスト宛ての要求だけに実値を差し替えます [sec-secrets] [secrets]。CLI は `--secret 'GITHUB_TOKEN@api.github.com'` の形で、ホストの環境変数への参照を保存し、sandbox の起動時に読みます（値の直接指定は拒否）[secrets]。TLS 検査が必須で、ゲストは microsandbox の CA を信頼する必要があります。curl・git・Node・Python には環境変数で設定され、JVM は別途の設定が要ります [issue-1689]。ゲスト内で署名に使う生の資格は、プレースホルダーでは置き換えられません [secrets]。`agent-gh` は呼び出しごとに fnox から GitHub App の鍵を取り出してインストールトークンを発行する（`scripts/agent-gh`）ため、鍵（`~/.config/pelway-agent/fnox.toml`・`~/.config/fnox/age.txt`）を `ro` でマウントして VM 内で fnox を動かす形になります。この構成は**未確認**で、鍵の露出は現行と変わりません。試すときは本番の鍵を VM に渡しません。`agent-gh` は試験用のリポジトリと試験専用の資格情報で確かめます（4）。opencode 自体を VM 内で動かす公式ガイドは、資格を VM 内の named volume に置きます [opencode-guide]。すると資格は同じ VM 内のコマンドから読めます。プレースホルダーをエージェントのセッションに使うと、要求がブロックされてセッションが続行できなくなる報告があります [issue-2014]。**T3 への乗せ方**: `msb exec <name> --stream -- opencode acp` で ACP の対話を続ける例が公式にあり [jetbrains-acp] [sandbox-cmd]、T3 の acpRegistry は `<commandPath> acp` を起動する（`docs/dev-environment.md`）ので、ラッパーで置き換える構成は成り立つ見込みです。T3 の MCP サーバーへの到達は**未確認**です。Devin CLI の公式ガイドはありません |
| 性能・キャッシュ | 同一ファイルシステムのため、`~/.cargo`・`~/.rustup`・`~/.local/share/mise`・`~/.cache/mise` 等のキャッシュはホストと共有して rw。mbx・hk 等は `XDG_CACHE_HOME` で sandbox 専用キャッシュに退避 [docker-sandboxes] | VM 内は独自のファイルシステムで、ホストのキャッシュは見えない。追加 workspace で共有マウントするか、sandbox 内に保持する。直接マウント workspace の virtiofs 読み取りキャッシュは既定で有効。cargo・mise・mbx のキャッシュをホストと共有させる構成は未確認 [docker-sandboxes] | 公称はゲストの起動が平均 100 ms 未満ですが、Apple M1 での値で Linux の公称値はありません [readme]。一般ユーザーの報告では、`msb create alpine:3` に約 2.7 秒かかった例があります（Intel Core i7-2600、v0.7.14）[issue-2127]。既定は 1 vCPU・512 MiB メモリーです [sandbox-overview]。同梱カーネルの `NR_CPUS` は x86_64 で 16 です [krunfw-readme]。ホストのキャッシュ（`~/.cargo`・`~/.rustup`・mise）を bind mount で共有できるか、共有したときのビルド時間は**未確認**です。bind mount で多数の小ファイルを扱うとホストとゲストの見え方が食い違う報告があり、ホストの `ulimit -n` を上げると解消したというコメントが付いています [issue-949]。bind mount への最初の `statfs` はマウント全体の走査を伴います [issue-1701]。`/tmp` が 512 MiB の tmpfs になるという公開中の報告と、ルートディスクを使うという文書が併存します [issue-1377] [volumes]。mbx のキャッシュとの組み合わせは**未確認**です |
| 条件 | 追加費用・契約・サインイン・テレメトリなし（bwrap パッケージのみ）[docker-sandboxes] | `sbx` CLI とローカル sandbox の計算資源は商用利用も含めて無償。ただし Docker アカウントでのサインインが必須。CLI 使用テレメトリを収集し、`sbx settings set telemetry.disabled true` で停止できる。プロプライエタリ（Docker Inc. ライセンス）[docker-sandboxes] | ライセンスは Apache-2.0 です [cargo-toml]。依存クレートは cargo-deny の許可リスト（Apache-2.0・MIT・BSD・ISC・MPL-2.0 等）に限定しています [deny-toml]。カーネルを同梱する libkrunfw は、Linux カーネルとパッチが GPL-2.0-only、ライブラリー部分が LGPL-2.1-only で、バイナリー配布にはカーネルのソースの添付が要ります [krunfw-readme]。イメージは Docker Hub・GHCR などの OCI レジストリから取得し、各イメージのライセンスに従います。ダイジェストは検証しますが、署名は検証しません [sec-fs]。ローカル利用にアカウントは不要です [cli-overview] [cloud-connect]。課金は managed cloud の利用時だけです [pricing]。利用状況の送信については、設定項目に該当するものがなく [configuration]、GitHub のコード検索（`telemetry`・`analytics`・`posthog`・`sentry` など）でも利用状況の送信に当たる箇所は見つかりませんでした。`telemetry` の一致は sandbox の起動進捗と metrics だけです。CLI のコマンド実装に書かれた外部ホストは、`msb self update` が呼ぶ GitHub Releases の API と、managed cloud を選んだときだけ使う API です [cli-sources]。これは取得した範囲からの判断で、全ソースの監査ではありません。README は beta であることを明記しています [readme]。v0.8.1 と v0.8.3 は、パッチ番号のリリースに破壊的変更を含みます [release-v0.8.1] [release-v0.8.3]。セキュリティ勧告は 14 件が公開され、critical 2 件・high 8 件を含みます。修正版はいずれも v0.7.12 までに出ています [advisories] |

## 3. 方針との関係

`docs/dev-environment.md` の「コンテナ（Docker を使わない）」には「Docker Desktop・Docker デーモンには依存しない」という方針があります。microsandbox はこの方針とぶつかりません。

- microsandbox は OCI イメージを自分で取得し、ホストの Docker デーモンを使いません [images-overview] [introduction]。
- 独自のイメージを渡すときは、Docker のローカルのイメージストアを直接読めないため、`docker save | msb load` か、OCI Image Layout のアーカイブ（BuildKit・Skopeo・ORAS などが作る）を使います [local-images]。Docker が要るのはイメージを Docker で作る場合だけです。Pelway は rootless Podman を使う方針なので、`podman build` と `podman save` で代用できる見込みですが**未確認**です（minisforum に Podman が導入済みかは今回確認していません）。
- VM の中で Docker を動かすガイドもありますが、ホストの Docker には触れません [docker-in-sandbox]。

方針を変える案は不要です。microsandbox を使わない場合も方針はそのままで、bwrap 方式を続け、bwrap に足りない部分（ネットワークの制御、資格情報を sandbox 内に置かない注入の仕組み）は内製の改善として別途検討します（`docs/research/docker-sandboxes.md` の (b) と同じです）。

## 4. 実際に試す段階の計画（実行はしない）

試すと決まった場合の、導入から片付けまでの手順と、hatai に頼む許可の一覧です。試すときは本番の環境と分けます。保存先は試行専用のディレクトリーにし、リポジトリーは試験専用のクローンを使い、資格情報は試験専用のものだけを渡します。導入と片付けは、既存のコマンドリンク（`~/.local/bin`）とシェルの設定ファイル（`.profile`・`.bashrc` など）に書き込まない手順にします。

### 手順の概略

1. 試行専用のディレクトリー `<MSB_TRIAL>`（例: `~/.microsandbox-trial`）を作り、GitHub Releases から手で導入します。`install.sh` は使いません。このスクリプトは、保存先を `MSB_HOME` で変えられても、既存の `~/.local/bin/msb`・`~/.local/bin/microsandbox` のシンボリックリンクを新しい導入に付け替え、リンクを作らない選択肢がありません [install-sh]。手順は `install.sh` の `install` 相当の部分だけを写します。`<版>` のリリースから `microsandbox-linux-x86_64.tar.gz` と `checksums.sha256` を取得し、`sha256sum -c` で照合し、失敗したら展開する前に止めます。展開して `msb` を `<MSB_TRIAL>/bin/msb` に `install -m 755` で置き、`<MSB_TRIAL>/bin/microsandbox` を `msb` へのシンボリックリンクにします。`libkrunfw.so.<版>` を `<MSB_TRIAL>/lib/` に置き、`libkrunfw.so.<ABI>` と `libkrunfw.so` のシンボリックリンクを張ります [install-sh]。`<MSB_TRIAL>` は `chmod 700` にします（umask が 002 だと保存先が group-writable になり、v0.7.12 以降はそのディレクトリーを保存先として拒否されます）[issue-2003]。sudo は不要で、シェルの起動ファイルは書き換えません。
   試行の必須設定として `<MSB_TRIAL>/config.json` に `{"allowed_backends": ["local"]}` を書きます。`MSB_HOME` が切り替えるのは保存先だけで、CLI の実行先（ローカルか microsandbox のクラウドか）は別に決まります。実行先は明示の `--local`・`--remote`、managed `active_profile`、`MSB_BACKEND`、`MSB_PROFILE`、保存済み `active_profile`、ローカル実行 の順で選ばれるため、クラウド利用時の環境設定と有効な資格情報が残るシェルではリモートが選ばれえます [cloud-connect]。`allowed_backends: ["local"]` はリモートでの実行自体を拒否しますが、実行中の接続は切り替えないため、実行先の固定は両方で行います（設定による拒否と、対応するコマンドでの `--local` の明示）。CLI が読む設定ファイルは `MSB_CONFIG_PATH` で指定したパスが優先され、未指定なら保存先の `config.json` です [config-path]。シェルに `MSB_CONFIG_PATH` が引き継がれていると `<MSB_TRIAL>/config.json` は読まれず、その設定に `home` や `paths` の指定があると `--local` であっても試行外の保存先に書き込みます。実行先の表示はローカルのままなので、起動やイメージ取得で書き込むまで気づけません。試行では `MSB_CONFIG_PATH=<MSB_TRIAL>/config.json` の絶対パスを明示します（または引き継いだ値を外します）。さらに管理設定 `managed.json`（Linux では `/etc/microsandbox/managed.json`）はユーザー設定と CLI・SDK の指定のどちらよりも優先されるため、これが存在しないか、`allowed_backends` や保存先（`home`・`paths`）を上書きしていないことを、最初の書き込み（イメージの取得と VM の起動）の前に確かめます [managed-config] [configuration]。
   以降の手順で `msb` と書くときは、`<MSB_TRIAL>/bin/msb` に `MSB_HOME=<MSB_TRIAL>` と `MSB_CONFIG_PATH=<MSB_TRIAL>/config.json` を前置して呼ぶものを指します（例: `MSB_HOME=<MSB_TRIAL> MSB_CONFIG_PATH=<MSB_TRIAL>/config.json <MSB_TRIAL>/bin/msb --local ps -a`）[versions-doc]。microsandbox の保存先は、空でない `MSB_HOME`、未設定なら `~/.microsandbox` の順に決まります [resolve-home]。`MSB_HOME` を引き継がないシェルでフルパスで呼ぶと、既定の `~/.microsandbox` に書き込みます。`--local` は実行先を選べるコマンド（sandbox・volume・snapshot 系と、引数なしの `context`）にだけ付けます。`doctor`・`pull`・`images`・`df`・`rmi`・`prune`・`self` など実行先を選べないコマンドに付けると、v0.8.3 は処理を始める前に拒否します。選べないコマンドも処理の前に実行先を解決するため、これらを呼ぶのは、同じ環境で引数なしの `msb context` がローカルを示すことを確かめた後だけにします（手順 3）[msb-selector]。
2. `sudo usermod -aG kvm hatai` で `kvm` グループに追加します。反映にはログインし直しが要ります [linux-trouble]。WSL ではディストロの再起動（`wsl --shutdown`）も要る可能性があります。
3. `msb doctor` で `/dev/kvm` の読み書きを確認します [linux-trouble]。`msb context`（引数なし）で CLI の実行先がローカルであることを確かめます（`msb context` はコマンドがどこで動くかを表示します）[cloud-connect]。リモートを向いていたら（または表示がエラーなら）手順を進めず、環境設定と資格情報が残らないシェルでやり直します。`--local` を付けると `msb context` は選ばれた実行先として必ずローカルを表示するため、この確認はフラグなしで行います [msb-selector]。`MSB_CONFIG_PATH` が試行の設定の絶対パスを指していること（または未設定で `<MSB_TRIAL>/config.json` が読まれること）と、管理設定 `/etc/microsandbox/managed.json` が無いか、`allowed_backends`・保存先（`home`・`paths`）を上書きしていないかもここで確かめます [config-path] [managed-config]。`doctor` と以降の `pull`・`df` など実行先を選べないコマンドはフラグなしで呼び、このとき確かめた環境でだけ実行します [msb-selector]。
4. `msb pull alpine` でイメージを取得します（Docker Hub などの OCI レジストリへの通信）[image-cmd]。
5. 資格情報を渡さずに確認します（下の (a)〜(d)）。
6. 試験用のリポジトリと試験専用の資格情報で、git の操作・`agent-gh`・エージェントの起動を確認します（下の (e)〜(g)）。
7. 片付けます。`msb --local ps -a` の一覧が試行で作った sandbox だけであることを確かめ、試行のものだけに `msb --local rm --force <名前>` で消します（`msb ps`・`msb rm` は選択中の実行先に届くため、一覧に試行外の VM が出るときは何も消さず止めます）[cloud-connect]。`msb df --verbose` は実行先を選べないコマンドなのでフラグなしで呼び、その前に引数なしの `msb context` がローカルを示すことを確かめます [msb-selector]。`msb df --verbose` で項目ごとの名前とパスを確認します（引数なしの `msb df` は種類別の集計だけで、試行で作った項目だけかは分かりません）[storage-df]。`<MSB_TRIAL>` の実体（`realpath`）と `ls -la` の中身が試行で作ったものだけであることを確認してから、`rm -rf -- <MSB_TRIAL>` を実行します。`msb self uninstall --yes` は使いません。このコマンドは保存先のディレクトリー全体の削除に加えて、`~/.local/bin` のコマンドリンクの削除と、ホストのシェル設定の古い記述の整理を行い、削除対象は実行ファイルの場所ではなく実行時の `MSB_HOME` で決まるためです [self-uninstall] [resolve-home]。`sudo gpasswd -d hatai kvm` でグループから外します。

### hatai に頼む許可の一覧

| # | 承認 | 内容 |
|---|---|---|
| 1 | **導入（方法と sudo の要否）** | 上の 1。sudo 不要。GitHub Releases への通信。試行専用のディレクトリーへの書き込みだけ（`install.sh`・`~/.local/bin`・シェル設定・本番の `~/.microsandbox` には触れない） |
| 2 | **`kvm` グループへの追加** | 上の 2。`sudo usermod -aG kvm hatai` と、反映のための再ログイン（WSL の再起動を含みうる） |
| 3 | **イメージの取得などの外部通信** | 上の 4。Docker Hub などの OCI レジストリへの通信（検証 (a) の `msb run alpine` による取得を含む）。独自イメージを作る場合は、ビルドツールの導入とその通信 |
| 4 | **試す範囲** | 本番の T3 スレッドでは使わない。リポジトリーは試験専用のクローン、保存先は試行専用のディレクトリーを使う。本番の資格情報（`~/.config/pelway-agent`・`~/.config/fnox`）は VM に渡さない。試験専用の資格情報は、置き場所だけでなく中身でも本番と別にする: GitHub は試験用のリポジトリにだけインストールした別の GitHub App の鍵を使い、pelway-dev・pelway-review の鍵は試験用の fnox 設定にも入れない（App の秘密鍵はその App の全インストール先のトークンを発行できるため、別の age 鍵で暗号化しただけの設定は試験専用とみなさない。`scripts/agent-gh` と `scripts/setup/agent-accounts.sh`）。モデルのキーは試行のために発行し、終わったら失効させる。(f) の検証は、T3 の接続の資格情報を VM に渡さず、本番の T3 の acpRegistry も変えない形に限る。`install.sh` と `msb self uninstall` は使わない |
| 5 | **サインイン・統計送信の扱い** | 不要です。ローカル利用にアカウントは要らず、統計送信も見つかっていません。試行では `allowed_backends` に `["local"]` を必須設定とし、実行先を選べるコマンド（sandbox・volume・snapshot 系と引数なしの `context`）にだけ `--local` を付けて、リモート（microsandbox のクラウド）での実行を拒否します。`MSB_CONFIG_PATH` には試行設定の絶対パスを指定し（引き継いだ値は使わない）、`/etc/microsandbox/managed.json` が許可先や保存先を上書きしていないことを最初の書き込みの前に確かめます [configuration] [cloud-connect] [config-path] [managed-config] [msb-selector] |
| 6 | **片付けの手順** | 上の 7。削除対象は試行で作成した VM と試行専用のディレクトリーに限る（一覧に試行外の VM が出るときは何も消さない）。`msb self uninstall` は使わない（コマンドリンクとシェル設定に触れ、削除対象が実行時の `MSB_HOME` で決まるため）。`sudo gpasswd -d hatai kvm` も含みます |

### 採否の判定材料にする検証

- (a) WSL2 の上で microVM が起動するか（`msb doctor`、`msb run alpine`）。`msb run` は、イメージが保存先に無ければレジストリから取得します [images-overview]
- (b) bind mount 上の挙動。ELF の `execve`（#2121）、symlink の作り替え（#2063）、実行ビット（#742）。試したあとホスト側の `git status` が汚れていないか
- (c) 試験専用のクローン（既存のリポジトリーの共有 git dir を使わない、独立した clone）に `git config extensions.worktreeConfig true` を設定し、その中で worktree を作ります。試験用クローンの両方の worktree で `git config --worktree` に無害な設定（例: `git config --worktree msb.trial true`）を保存し、`config.worktree` の実体を作ります。`git worktree add` は元の worktree に `config.worktree` がなければコピー先のファイルを作らず、`git rev-parse --git-path config.worktree` が返すパスにも実体がないままでは、`ro` で重ねる検査対象ができないためです [git-worktree]。worktree と共有 git dir をホストと同じ絶対パスでマウントし、`ro` の重ねを確かめます。エージェントを起動する前に、ゲストから worktree 固有の `config.worktree`（`git rev-parse --git-path config.worktree` が示す `worktrees/<id>/config.worktree`）を読めることを確かめてから、共有 git dir の `config`・`config.worktree`・`hooks/` と、worktree 固有の `config.worktree`（`worktrees/<id>/config.worktree`）への書き込みが拒否されることを確認します。`extensions.worktreeConfig` を有効にすると、Git は worktree 固有の `worktrees/<id>/config.worktree`（共有 git dir の下）を読むため、一覧の `config.worktree` だけでは保護を確かめられません [git-worktree]。現行の bwrap が守っている対象と、worktree 固有の設定ファイルです。拒否されなければ、そこで止めて残りの検証はしません
- (d) 既定のネットワークで crates.io・GitHub・一般のサイトに届くか。ホストとの接続を確かめるときは試行専用のサービスとその確認だけに使う VM で行い、その VM は後続のエージェント起動やビルド（(e)〜(g)）には使いません（接続確認に使った VM で動くコードも、ホストの試験対象サービスに届くため）。`host`（または `private`・`all`）の許可はこの確認に限ります。T3 の MCP サーバーへの到達経路（`host` の許可、`--vsock`）は成り立つか
- (e) (c) が通った後に、試験用のリポジトリに対して `git status`・`git commit`・`scripts/agent-gh dev git push` と `agent-gh-resign` が通るか。既定のネットワークポリシー（ホスト自身と私網を拒否）で動かす VM で行います（(d) で `host` などを許した VM は使いません）。資格情報は試験専用のもの（`PELWAY_AGENT_FNOX_CONFIG` で指す試験用の fnox 設定と鍵）を使い、本番の `~/.config/pelway-agent` と `~/.config/fnox` は使いません。試験用の fnox 設定には、試験用のリポジトリにだけインストールした別の GitHub App の秘密鍵を入れます。pelway-dev・pelway-review の鍵は入れません（App の鍵は全インストール先のトークンを発行できるため、pelway-dev の鍵を別の age 鍵で暗号化した設定では VM 内のコードが hatai/Pelway への書き込みトークンを得られます）
- (f) `msb exec <name> --stream -- <command>` が VM 内のコマンドの stdio をホスト側に中継できるか。(e) と同じく既定のネットワークポリシー（ホスト自身と私網を拒否）で動かす VM で行います。T3 の acpRegistry の `commandPath` に置く形（`msb exec <name> --stream -- opencode acp` を呼ぶラッパー）を想定した確認ですが、本番の T3 の設定は変えず、ACP のクライアントを直接呼んで確かめます。T3 の接続の資格情報（`T3_ACP_MCP_AUTHORIZATION` など。現行の bwrap はエージェントのコマンドから外しています）を VM に渡す形や、本番の T3 の acpRegistry を変える検証は、別の許可が要ります。VM 内の opencode でモデルを呼ぶ場合は、試行のために発行して終わったら失効させるキーを使います
- (g) `mise run verify` が VM 内で通るか、時間はどれくらいか。(e) と同じく既定のネットワークポリシー（ホスト自身と私網を拒否）で動かす VM で行います

## 5. チームのおすすめ

**条件付きで試す**。`sbx` より先に microsandbox を試します。

microsandbox を選ぶ理由:

- Pelway の方針と条件に合います。Docker に依存せず、アカウントもサインインも要らず、ライセンスは Apache-2.0 です。利用状況の送信も見つかりません。`sbx` が持つ Docker Inc. のプロプライエタリ製品への依存・Docker アカウント・テレメトリ（opt-out）という課題がありません [docker-sandboxes]。
- 分離は bwrap より強い面があります。ゲストは独立したカーネルで動き、ホストのファイルシステムはマウントしたもの以外は見えず、ネットワークはホスト側のポリシーで検査されます [isolation] [sec-net]。
- minisforum は glibc と KVM の条件を満たし、残るは `kvm` グループへの追加だけです（`/dev/kvm` の現在の読み書き可否と、グループ追加で十分かは、このスレッドからは見えないため未確認です）。

試すのを待つ理由:

- upstream は WSL2 を試験対象外とし、優先度も下げています [issue-920] [issue-47]。minisforum で動くかは起動するまで分かりません。
- ホストと共有するフォルダーの挙動に公開中の不具合があります（2 の git の行）。Pelway は worktree を T3 とホストで共有し、エージェントは git 操作とビルドをするため、この不具合は現行の運用に直接当たります。

留意点:

- ゲストとホストの境界は、ハイパーバイザーに加えて、ホスト側の仲介コード（ファイル・ネットワーク・snapshot の処理）が担います。公開中のセキュリティ勧告には、ゲストが制御できるデータを入口にホストのファイルの読み書きやネットワークポリシーの迂回が起きるものが含まれます [advisories]。bwrap と比べるときは、この部分の成熟度も分離の強さに数える必要があります。
- WSL2 の microVM が起動しなかった場合も、`sbx` が起動しないとは限りません。仮想化の実装が別だからです。

条件:

1. bind mount 上の ELF の `execve` の回帰（#2121）が修正リリースに載ること。symlink の挙動（#2063）は修正されるか、試す際に symlink を作り替える操作を避けられること。
2. `kvm` グループへの追加など、4 の許可が下りること。
3. 検証 (a) で microVM が起動しなければ、そこで終わりにして見送ります。

`sbx` と比べると、両方とも WSL2 は best-effort で、`kvm` グループが要ります。`sbx` は「ホストの git worktree を直接マウントすると git が使えない」という文書上の制約があり、回避の記載がありません。microsandbox は普通のリポジトリーでは git が使える例があり [opencode-guide]、残るは worktree の検証と不具合の修正待ちです。方針への適合でも microsandbox が上です。このため microsandbox を先にします。

WSL2 で動くかだけを先に知りたい場合は、検証 (a) だけで済みます。上の条件を待たずに試せます。ただし (a) にも、導入と `kvm` グループの追加に加えて、イメージの取得が要ります。`msb run alpine` が、イメージを保存先に持たなければ Docker Hub から取得するためです [images-overview]。4 の許可の #1〜#3 が要ります。

## 出典

- [docker-sandboxes] 本リポジトリ `docs/research/docker-sandboxes.md`（`sbx` の列の出典。bwrap 方式の列は同文書が `scripts/agent-sandbox/sandbox-shell` と `docs/dev-environment.md` に基づくとしている）
- [git-worktree] https://github.com/git/git/blob/v2.51.0/Documentation/git-worktree.adoc#L322-L333 （`extensions.worktreeConfig` と worktree 固有の `config.worktree`）。`git worktree add` が `config.worktree` を複製しない条件は https://github.com/git/git/blob/v2.51.0/builtin/worktree.c#L361-L377
- [readme] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/README.md （beta の明記・要件・導入方法・起動時間の脚注）
- [linux-trouble] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/troubleshooting/linux.mdx （glibc・`/dev/kvm`・`kvm` グループ・入れ子の仮想化）
- [windows-trouble] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/troubleshooting/windows.mdx （Windows 版がプレビュー・WHP）
- [introduction] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/getting-started/introduction.mdx
- [cli-overview] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cli/overview.mdx （導入先・アカウント不要）
- [install-sh] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/scripts/install.sh
- [cli-management] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cli/management.mdx （`msb self` の update・uninstall・doctor）
- [self-uninstall] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/lib/commands/self_cmd.rs#L2022-L2103 （`msb self uninstall` の実装。Linux では `~/.local/bin` のコマンドリンクの削除・シェル設定の古い記述の整理・保存先全体の削除を行う。`remove_public_command_links`・`clean_legacy_shell_config`・`resolve_base_dir` は同ファイル）
- [resolve-home] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/utils/lib/lib.rs#L159-L178 （`resolve_home()` の保存先の選択規則。空でない `MSB_HOME`、未設定なら `~/.microsandbox`、home が無ければ `./.microsandbox` の順）
- [storage-df] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/lib/commands/storage.rs#L138-L164 （`msb df` の種類別の集計と、`--verbose` の項目ごとの名前とパス）
- [versions-doc] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/getting-started/versions.mdx （`MSB_HOME` を分けた試用と、バイナリーをフルパスで呼ぶ使い方）
- [configuration] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/configuration.mdx （`~/.microsandbox/config.json` の設定項目・`allowed_backends`）
- [isolation] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/security/isolation.mdx
- [sec-fs] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/security/filesystem.mdx
- [sec-net] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/security/network.mdx
- [sec-secrets] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/security/secrets.mdx
- [net-overview] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/networking/overview.mdx
- [dns] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/networking/dns.mdx
- [host-sockets] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/networking/host-sockets.mdx
- [volumes] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/sandboxes/volumes.mdx
- [sandbox-overview] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/sandboxes/overview.mdx
- [secrets] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/sandboxes/secrets.mdx
- [images-overview] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/images/overview.mdx
- [image-cmd] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cli/image-commands.mdx
- [local-images] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/how-to/docker/local-images.mdx
- [docker-in-sandbox] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/how-to/docker/docker-in-sandbox.mdx
- [opencode-guide] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/how-to/agents/opencode.mdx
- [jetbrains-acp] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/how-to/development/jetbrains-acp.mdx
- [sandbox-cmd] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cli/sandbox-commands.mdx （`msb exec --stream`）
- [cloud-connect] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cloud/connect.mdx
- [pricing] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/cloud/pricing.mdx
- [cargo-toml] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/Cargo.toml
- [deny-toml] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/deny.toml
- [cli-sources] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/lib/commands/self_cmd.rs 、 https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/lib/commands/context.rs
- [krunfw-readme] https://github.com/superradcompany/libkrunfw/blob/f0493b6f1873ed6031bc0bb87568db0095a6d3b6/README.md （ライセンスの説明・`NR_CPUS`）
- [release-v0.8.3] https://github.com/superradcompany/microsandbox/releases/tag/v0.8.3
- [release-v0.8.1] https://github.com/superradcompany/microsandbox/releases/tag/v0.8.1
- [release-v0.8.2] https://github.com/superradcompany/microsandbox/releases/tag/v0.8.2
- [advisories] https://github.com/superradcompany/microsandbox/security/advisories
- [issue-47] https://github.com/superradcompany/microsandbox/issues/47 （Windows 対応。WSL2 非推奨の回答）
- [issue-920] https://github.com/superradcompany/microsandbox/issues/920 （WSL2 での KVM クラッシュ報告。WSL は best-effort・優先度低下の回答）
- [issue-742] https://github.com/superradcompany/microsandbox/issues/742 （ゲストの mode ビットがホストに届かない）
- [issue-743] https://github.com/superradcompany/microsandbox/issues/743 （ゲストが作った symlink がホストでは通常のファイル。意図的な設計としている回答）
- [issue-949] https://github.com/superradcompany/microsandbox/issues/949 （passthrough mount でのファイルの見え方の食い違い）
- [issue-1095] https://github.com/superradcompany/microsandbox/issues/1095
- [issue-1290] https://github.com/superradcompany/microsandbox/issues/1290 （bind mount したソケットはホストに届かない・`--vsock`）
- [issue-1377] https://github.com/superradcompany/microsandbox/issues/1377 （`/tmp` の tmpfs）
- [issue-1551] https://github.com/superradcompany/microsandbox/issues/1551 （ホスト側の変更がゲストに届くまでの遅れ）
- [issue-1689] https://github.com/superradcompany/microsandbox/issues/1689 （TLS 検査の CA の信頼）
- [issue-1701] https://github.com/superradcompany/microsandbox/issues/1701 （bind mount への最初の `statfs` がマウント全体を走査する）
- [issue-2003] https://github.com/superradcompany/microsandbox/issues/2003 （umask 002 で作った `~/.microsandbox` が拒否される）
- [issue-2014] https://github.com/superradcompany/microsandbox/issues/2014 （プレースホルダーのブロックでエージェントのセッションが止まる）
- [issue-2063] https://github.com/superradcompany/microsandbox/issues/2063
- [issue-2121] https://github.com/superradcompany/microsandbox/issues/2121 （bind mount 上の ELF の `execve` が `EACCES`）
- [issue-2127] https://github.com/superradcompany/microsandbox/issues/2127 （x86-64-v3 のゲストカーネル）
- [pr-1448] https://github.com/superradcompany/microsandbox/pull/1448 （bind mount の deny list。未マージ）
- [pr-2125] https://github.com/superradcompany/microsandbox/pull/2125 （#2121 の修正。未マージ）
- [config-path] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/sdk/rust/lib/config/mod.rs#L821-L827 （`config_path()`。`MSB_CONFIG_PATH` が優先）
- [managed-config] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/docs/enterprise/managed-configuration.mdx 、 https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/sdk/rust/lib/config/persistence.rs#L212-L222 （`managed.json` の優先順位とパス。Linux は `/etc/microsandbox/managed.json`）
- [msb-selector] https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/bin/main.rs#L382-L395 、 https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/bin/main.rs#L833-L890 、 https://github.com/superradcompany/microsandbox/blob/0dad6d434ea94fbf74c9f02a88cd9ff12f8420ff/crates/cli/bin/main.rs#L933-L959 （`--local`/`--remote` を受け付けるコマンドの範囲と、実行先の解決・バックエンドを要しないコマンド）
