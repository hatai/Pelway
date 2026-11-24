# Docker Sandboxes（sbx）を実装担当の sandbox に使えるかの調査

Issue #689。`sbx` は導入せず、公式文書・`docker/sbx-releases` の公開 Issue・ホストの読むだけのコマンドで確認した。「minisforum で確認」の項目のコマンドと日時（JST）は `docs/worklogs/issue-689.md` に記録する。

## 結論

**条件付きで試す** を推す。minisforum の WSL2 は KVM の条件をほぼ満たしており（残る未達は `kvm` グループ未所属のみ）、Linux 版の WSL2 内利用は「best-effort」ながら実績の報告もある。一方で Pelway の運用との適合には、ホストの git worktree を直接マウントすると git が使えないという公式文書上の制約、`agent-gh`（fnox+age 鍵）の経路、T3 worktree スレッドの起動方式との相性という、導入して試さないと結論の出ない不確かさが残る。試すために hatai に頼む承認は 4 に列挙する。

## この文書の前提と読み方

- 「確認」は公式文書か `docker/sbx-releases` の公開 Issue に記載があるという意味で、実機での動作確認ではない。「未確認」は今回の資料で裏付けられなかった項目で、機能が無いという意味ではない（docs/research/ の他文書と同じ流儀）。
- bwrap 方式の列は `scripts/agent-sandbox/sandbox-shell` と `docs/dev-environment.md` の「実装担当の sandbox（agent-sandbox）」の記述に基づく。
- 用語：`sbx` は Docker Sandboxes の CLI。エージェントを microVM（軽量の仮想マシン）の中で動かす Docker Inc. の製品で、ライセンスはプロプライエタリ（`docker/sbx-releases` の LICENSE）。KVM は Linux カーネル内蔵のハイパーバイザ。bwrap（bubblewrap）は mount 名前空間でファイルアクセスを制限するツールで、現行の実装担当 sandbox の基盤。

## 1. 対応環境：WSL2 の minisforum で `sbx` のローカル sandbox が動く見込み

### 公式の条件と minisforum の状態の突き合わせ

公式文書の Linux の条件は「Ubuntu 24.04 以降」「KVM 仮想化が CPU で有効で、利用者が `kvm` グループに所属」。仮想マシン内で使う場合は入れ子の仮想化（nested virtualization）が要ると明記されている [install]。

| 公式の条件 | minisforum の状態 | 判定 |
|---|---|---|
| Ubuntu 24.04 以降 | Ubuntu 24.04.5 LTS（WSL2。`uname -a` は `6.18.58.1-microsoft-standard-WSL2`） | 合う |
| KVM 仮想化が有効 | `kvm_amd`・`kvm` モジュール読み込み済み（`lsmod`、minisforum で確認。カーネルモジュール一覧は名前空間で分かれないためホストの状態を示す）。`/dev/kvm` が `root:kvm`・`crw-rw----` で存在（Issue #689 本文のホスト側記録） | 合う（下記のグループを除く） |
| 利用者が `kvm` グループに所属 | `id -nG` の出力は `hatai nogroup` で `kvm` を含まない（minisforum で確認） | **未達。`usermod -aG kvm` が要る** |
| VM 内では入れ子の仮想化 | WSL2 は amd64 で入れ子の仮想化が既定で有効（Microsoft 公式リリースノート Build 20175）[ms-wsl-rel]。`kvm_amd` の読み込みと `/dev/kvm` の存在が有効であることの証拠 | 追加の Windows 側設定は不要の見込み |

補足：上の確認は実装担当の sandbox（bwrap）の内側から行った。sandbox 内の `/dev` は最小限の構成で `/dev/kvm` を含まないため、スレッド内からは `/dev/kvm` の有無を確認できない。これは sandbox のマウント構成による見え方の差で、ホスト側の記録（`/dev/kvm` 存在）と矛盾しない。

### WSL2 は正式な対応環境か

- 公式文書の対応環境（macOS・Windows 11・Ubuntu 24.04 以降）に WSL2 は載っていない [install]。
- `docker/sbx-releases` の Issue #397（closed）で、メンバー（`kiview`、author_association: MEMBER）が「Windows 版を入れて `sbx.exe` を使うのが推奨で、WSL に Linux 版を入れるのは best-effort」と回答している [issue-397]。
- 実績の報告はある。Issue #787 の報告者は WSL2（Arch、入れ子の仮想化あり・`networkingMode=mirrored`）で Linux 版を日常的に使っていたと書いている（同 Issue は clone-mode の remote が `lo` に bind される回帰で、closed・completed）[issue-787]。
- WSL 固有の不具合は複数報告されている：`--clone` が WSL で作れない（#491）、ログイン資格の保存失敗（#180）、ssh-agent の転送（#391）、mirrored モードでの loopback ルーティング（#787・#822）。いずれも closed。
- 別経路として Windows 版を入れて `\\wsl.localhost` のワークスペースを渡す案は、chmod/symlink が失敗する障害（#598、closed）と、Windows 側の sbx デーモンが WSL2 の VM 起動を壊す報告（#900、open・Windows 10 + Hypervisor Platform）があり、この調査では採らない。

**見込みの結論**：minisforum の WSL2 でローカル sandbox が動く見込みはある（KVM は揃っており、残る操作は `kvm` グループへの追加だけ）。ただし正式サポート外の「best-effort」であり、WSL 固有の不具合に当たる可能性は残る。

## 2. 今の bwrap 方式との比べ

| 観点 | bwrap 方式（現行） | `sbx` |
|---|---|---|
| ファイル | セッションの worktree・共有 git dir・ツールキャッシュのみ書き込み可。`/home`・`/etc` 等は read-only。秘匿パス（`~/.ssh`・`~/.gnupg`・`~/.docker`・`~/.config/gh`・`~/.config/opencode`・`~/.config/devin`・`~/.t3/userdata` 等）は tmpfs で隠す。worktree 内 symlink 越しの脱走は opencode プラグインの実体パス検査で防ぐ（`scripts/agent-sandbox/sandbox-shell`、`docs/dev-environment.md`） | microVM で、エージェントは VM 内のファイルシステムしか見えず、ホスト側は workspace として渡したパスだけが virtiofs 越しに見える（`sbx run` はカレントディレクトリを既定で rw マウント）[isolation] [usage]。`~/.ssh` 等の秘匿パスはマウントされないため到達不能で、tmpfs による遮蔽より強い。workspace 外への symlink は辿らない [isolation]。注意点は 2 つ：直接マウントは workspace 全体が rw で `.env` や未追跡ファイル・`.git/hooks` まで含む、workspace 内ファイルのハードリンクはワークスペース外のリンク先も書き換え得ると公式が警告 [isolation] |
| ネットワーク | 遮断しない。ホストの経路をそのまま使う（#151 で hatai が「許可リストにしない」と指示） | 外向き TCP は全てホスト上のプロキシ経由でポリシー評価される。preset に Open（`allow-all`、全 TCP 許可）があり、許可リストにしない方針とは TCP の範囲で両立する [governance-local]。ただし UDP は experimental の egress を有効化しないと出られず、ICMP は常に遮断で policy でも開けられない。DNS はポリシーを評価する sandbox 内の内部リゾルバが担う [isolation] |
| 資格情報 | `scripts/agent-gh` が呼び出しごとに fnox+age 鍵（`~/.config/pelway-agent/`、sandbox 内では read-only 可読）で GitHub App のトークンを発行し、`GH_TOKEN` を子にだけ渡す。モデル資格は devin の wrap 時のみ `~/.config/devin`・`~/.local/share/devin` を devin プロセスに可視にし、コマンドにはマスクする | ホスト側プロキシが宛先に応じて実資格を注入し、sandbox には sentinel（仮の値）しか見せない。`sbx secret set`（OS のキーチェーン。Secret Service が無い headless/WSL では `~/.config/com.docker.sandboxes` のファイルに退避）・`--command`（ホストでコマンドを実行して値を得る）・`--ref`（1Password/AWS Secrets Manager）・OAuth の経路がある [credentials] [faq]。`github` サービスは `GH_TOKEN`・`GITHUB_TOKEN` を api.github.com・github.com 等への要求に注入する [credentials]。モデル資格は agent ごとの service で注入する（opencode は openai/anthropic/google/xai 等、devin は初回に sandbox 内で OAuth サインイン後、ホスト管理の資格をプロキシ注入）[agents-opencode] [agents-devin]。`agent-gh` そのものは、VM 内に fnox と age 鍵が無いためそのままでは動かない。置き換え案は 2 つで、どちらも未確認：(a) `sbx secret set github --command '<ホストで pelway-dev トークンを発行するコマンド>'` でプロキシ注入に乗せる、(b) `~/.config/pelway-agent` を追加 workspace として read-only マウントし、VM 内に fnox を導入する |
| git | worktree の `.git` ポインタ先の共有 git dir（`git rev-parse --git-common-dir`）を rw で bind し、`config`・`config.worktree`・`hooks/` には read-only を重ねて url rewrite・hook 設置を防ぐ。push 後は `agent-gh-resign` が署名を作り直す流れが壊れない（`scripts/agent-sandbox/sandbox-shell`） | 文書上の既知の制約：**ホストの git worktree を直接マウントすると `.git` ポインタを解決できず、エージェントは git を使えない**（clone・commit・status 不可）[workflows-git]。`--clone` も main ではない worktree からは拒否される [usage]。回避案（共有 git dir を同じ絶対パスに追加 workspace としてマウントする）は文書に記載が無く未確認。仮に通ったとしても git dir 全体が rw になり、`config`・`hooks` に read-only を重ねる bwrap の保護を再現する手段が文書上見当たらない（sandbox 内から git の設定や hook を書き換えられ、他の worktree・ホスト側の git 操作にも効く） |
| 性能・キャッシュ | 同一ファイルシステムのため、`~/.cargo`・`~/.rustup`・`~/.local/share/mise`・`~/.cache/mise` 等のキャッシュはホストと共有して rw。mbx・hk 等は `XDG_CACHE_HOME` で sandbox 専用キャッシュに退避 | VM 内は独自のファイルシステムで、ホストのキャッシュは見えない。追加 workspace で共有マウントするか、sandbox 内に保持する（パッケージ・イメージ・ファイルは stop/restart で持続し、`sbx rm` で消える）[usage] [architecture]。直接マウント workspace の virtiofs 読み取りキャッシュは既定で有効 [architecture]。cargo・mise・mbx のキャッシュをホストと共有させる構成（virtiofs 越しの rw 共有、mbx のハードリンクキャッシュとの相性）は未確認。sandbox 内キャッシュに閉じる案は、sandbox ごとに Rust toolchain を入れ直す初回コストが重い見込み |
| 費用と条件 | 追加費用・契約・サインイン・テレメトリなし（bwrap パッケージのみ） | `sbx` CLI とローカル sandbox の計算資源は商用利用も含めて無償 [overview]。ただし Docker アカウントでのサインイン（`sbx login`、ブラウザ OAuth）が必須 [install] [faq]。CLI 使用テレメトリ（実行コマンド・成否・所要時間、サインイン中は Docker ユーザー名を含む。プロンプトやコードは含まないと明記）を収集し、`sbx settings set telemetry.disabled true` または `SBX_NO_TELEMETRY=1` で停止できる [faq]。組織ポリシーの中央管理は別の有償サブスクリプション [overview]。プロプライエタリ（Docker Inc. ライセンス）[sbx-releases] |

## 3. 方針との関係

`docs/dev-environment.md` の「コンテナ（Docker を使わない）」には「Docker Desktop・Docker デーモンには依存しない」という方針がある。

- **字面では衝突しない**：公式文書は「You don't need Docker Desktop or Docker Engine to use `sbx`」と明記しており、`sbx` はホストの Docker デーモンに依存しない（Linux では KVM で microVM を動かす。導入も `REPO_ONLY=1` で Docker Engine を入れずに `docker-sbx` パッケージだけを入れる経路がある）[install]。sandbox 内で動く Docker Engine は VM の内側の話で、ホスト方針には触れない [isolation]。
- **方針の意図との距離**：依存は消えるのではなく、Docker Inc. のプロプライエタリ製品＋Docker アカウント＋テレメトリ送信（opt-out 可）という別種の依存に置き換わる。方針が「ホストに Docker デーモンを常駐させない・Docker Desktop の重さとライセンスを避ける」という意図なら両立するが、「Docker 社製品への依存を避ける」意図も含むなら衝突する。

2 つの案：

- **(a) 方針を変える案**：「Docker Desktop・Docker デーモンには依存しない」を「ホストの Docker Desktop・Docker デーモンには依存しない」と明確化し、KVM ベースでホストの Docker を要らない `sbx` を例外として許容する。併せて、プロプライエタリ製品への依存と Docker アカウント必須・テレメトリの扱いを方針の文に追記する。
- **(b) `sbx` を使わない案**：方針をそのまま維持して `sbx` は導入せず、bwrap 方式に足りない部分（ネットワーク制御、資格情報を sandbox 内に置かない注入の仕組み）は内製の改善として別途検討する。

## 4. 実際に試す段階の計画（実行はしない）

試すと決まった場合の、導入から片付けまでの手順と、hatai に頼む承認の一覧。

### 手順の概略

1. `curl -fsSL https://get.docker.com | sudo REPO_ONLY=1 sh` で Docker の apt リポジトリだけを登録し（Docker Engine は入れない）、`sudo apt install docker-sbx` で導入する。または `docker/sbx-releases` の `.deb` を `sudo apt install ./<file>.deb` で入れる [install] [sbx-releases]
2. `sudo usermod -aG kvm hatai` で `kvm` グループに追加し、反映のために WSL のセッションを取り直す（`newgrp kvm` はそのシェルだけの回避で永続しない。グループ変更の反映に WSL distro の再起動が要る場合は `wsl --shutdown` も承認対象）[install]
3. `sbx login` で Docker アカウントにサインインする（ブラウザでの OAuth）[install]
4. テレメトリを止める方針なら `sbx settings set telemetry.disabled true` と `sbx daemon restart`（または環境変数 `SBX_NO_TELEMETRY=1`）[faq]
5. 検証（採否の判定材料。後述）
6. 片付け：`sbx rm`・`sbx prune` で sandbox を消し、`sudo apt remove docker-sbx` と apt リポジトリの登録解除、`sudo gpasswd -d hatai kvm`、`~/.config/com.docker.sandboxes` と sandbox ストレージの削除。Docker アカウント自体の扱いは hatai の判断

### hatai に頼む承認の一覧

| # | 承認 | 内容 |
|---|---|---|
| 1 | **sudo での導入** | `get.docker.com` スクリプト（`REPO_ONLY=1`）または `.deb` の `sudo apt install` で `docker-sbx` を入れる |
| 2 | **`kvm` グループへの追加** | `sudo usermod -aG kvm hatai` と反映（WSL セッションの再ログイン。distro 再起動が要れば `wsl --shutdown` も） |
| 3 | **Docker のサインイン** | `sbx login`（Docker アカウントの OAuth。無ければアカウント作成も） |
| 4 | **テレメトリの扱い** | 送るか、`sbx settings set telemetry.disabled true`／`SBX_NO_TELEMETRY=1` で止めるかの方針決定 |
| 5 | **片付けの手順** | 上記 6 の削除・グループ脱退を実施してよいか |

### 採否の判定材料にする検証

- (a) WSL2 内で microVM が起動するか（`sbx diagnose`、`sbx run --name verify shell`）
- (b) Pelway の worktree を直接マウントし、共有 git dir を同じ絶対パスに追加 workspace として渡して、sandbox 内で `git status`・`git commit` が通るか（2 の表の未確認点。ここが潰れると T3 worktree 運用に乗らない）
- (c) Open preset で crates.io・GitHub 等の TCP 通信が通るか（許可リストにしない方針との両立の実確認）
- (d) `sbx secret set github --command '<pelway-dev のトークンを発行するコマンド>'` で、sandbox 内の `gh` が pelway-dev 名義で api.github.com に届くか
- (e) `agent-gh` を sandbox 内で動かす経路（fnox・age 鍵の read-only マウント＋VM 内への fnox 導入）が作れるか、または (d) のプロキシ注入で代替できるか
- (f) T3 の起動方式との接合（`sbx exec` や SSH 接続を T3 の providerInstance の commandPath 相当から呼べるか）が構造として成り立つか

## 5. チームのおすすめ

**条件付きで試す**。

理由：

- 分離の強度は bwrap（mount 名前空間、カーネル共有）より microVM（ハイパーバイザ、独立カーネル）の方が高く、資格情報のプロキシ注入（実値を sandbox に見せない）は「秘密を sandbox 内に置かない」方向として現行（`~/.config/pelway-agent` の read-only 可読）より堅い [isolation] [credentials]。
- 一方で、2 の表の git の行のとおり、T3 worktree スレッド運用の根幹（worktree＋共有 git dir での commit と `agent-gh` の流れ）に、公式文書上の制約と文書に無い回避案がある。ここが潰れると採用の理由が無くなるため、採否を決めるのに十分な情報は導入せずには得られない。
- WSL2 が best-effort であるリスクは残るが、minisforum は KVM の条件を満たしており、失敗しても導入と片付けのコストで済む。

条件：`kvm` グループ追加・sudo・サインインの承認が下りたら、4 の検証 (a)〜(c) を先に行う。(b)（worktree＋git dir で git が通る）と (c)（Open preset で必要な TCP が通る）が両方通れば採用検討（(d)〜(f) と比較表の残りの未確認点の解消）に進み、どちらかが潰れた時点で採用は見送って bwrap 方式の改善に回す。

## 出典

- [install] https://docs.docker.com/ai/sandboxes/install/ （対応環境・導入手順・kvm グループ・入れ子の仮想化・サインイン）
- [get-started] https://docs.docker.com/ai/sandboxes/get-started/ （workspace の rw マウント・preset の選択・Docker daemon 不要の明記）
- [overview] https://docs.docker.com/ai/sandboxes/ （無償・有償の範囲、対応 agent 一覧への導線）
- [agents] https://docs.docker.com/ai/sandboxes/agents （対応 agent：Claude Code・Codex・Copilot・Cursor・Devin・Docker Agent・Droid・Kiro・OpenCode・Shell）
- [agents-opencode] https://docs.docker.com/ai/sandboxes/agents/opencode （opencode の資格情報と provider）
- [agents-devin] https://docs.docker.com/ai/sandboxes/agents/devin （devin の初回 OAuth と起動コマンド `devin --permission-mode dangerous --respect-workspace-trust=false`）
- [architecture] https://docs.docker.com/ai/sandboxes/architecture/ （microVM・workspace の passthrough・virtiofs キャッシュ・永続化）
- [isolation] https://docs.docker.com/ai/sandboxes/security/isolation/ （分離の 5 層・直接マウントの警告・ネットワーク分離・UDP/ICMP・DNS）
- [credentials] https://docs.docker.com/ai/sandboxes/configuration/credentials/ （プロキシ注入・`sbx secret set`・`--command`/`--ref`・WSL/headless でのファイル退避・github サービスの注入先ドメイン）
- [governance-local] https://docs.docker.com/ai/sandboxes/governance/local/ （network preset：Open/Balanced/Locked Down）
- [usage] https://docs.docker.com/ai/sandboxes/usage/ （clone mode の制約・複数 workspace・`sbx exec`・永続化・`--rm`）
- [workflows-git] https://docs.docker.com/ai/sandboxes/workflows/git/ （ホスト worktree マウントでは git が使えない明記・clone mode・sandbox remote）
- [faq] https://docs.docker.com/ai/sandboxes/faq/ （サインインの理由・テレメトリの内容と停止方法・headless/WSL での鍵の退避）
- [troubleshooting] https://docs.docker.com/ai/sandboxes/troubleshooting/ （`sbx diagnose`・SSH 等の非 HTTP TCP の扱い）
- [sbx-releases] https://github.com/docker/sbx-releases （README・手動導入・LICENSE がプロプライエタリであること）
- [issue-397] https://github.com/docker/sbx-releases/issues/397 （メンバーによる「WSL の Linux 版は best-effort」の回答）
- [issue-787] https://github.com/docker/sbx-releases/issues/787 （WSL2 での日常利用報告と mirrored モードの回帰）
- [ms-wsl-rel] https://learn.microsoft.com/en-us/windows/wsl/release-notes （Build 20175：amd64 で WSL2 の入れ子の仮想化が既定で有効）
- リポジトリ内：`scripts/agent-sandbox/sandbox-shell`、`scripts/agent-gh`、`docs/dev-environment.md`（「実装担当の sandbox（agent-sandbox）」「コンテナ（Docker を使わない）」）
