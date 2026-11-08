# 05 3つのOSでのローカル開発・テスト・配布

**今回は設計だけ**。環境の準備・インストール・登録はしていない。環境がなくてもこの文書は有効で、準備は濱田さんの承認後。

## 1. 基本の形
- **GUIはホストOSでビルドして動かす**。GPU、IME、通知、クリップボード、スクリーンリーダーはホストのAPIなので、コンテナの中では検証できない。
- **バックエンド・ツール・サンドボックスはLinuxコンテナ**。3つのOSで同じイメージを使う。
- ホストとコンテナは `localhost` のポートだけでつなぐ。

## 2. OSごと

| | Windows | macOS | Linux |
|---|---|---|---|
| GUIのビルド/実行 | Windows側で Rust（MSVCツールチェーン）。WSLの中ではビルドしない | macOSで Rust ＋ Xcodeのコマンドラインツール | ネイティブ |
| バックエンド | **WSL 2** のLinux内でコンテナ（Docker Desktop のWSL 2バックエンド、またはWSL内のDocker Engine/Podman） | **Apple container**（macOS 26・Apple silicon） | Podman または Docker Engine |
| ブラウザ版 | Windowsのブラウザで `localhost` | Safari/Chrome | ネイティブ |
| ホストでしかできない工程 | IMEとNVDAの試験、Windows通知、コード署名、インストーラー作成 | 日本語入力とVoiceOverの試験、通知の許可、署名・公証（notarization） | fcitx5/IBusとOrcaの試験、X11/Waylandの差 |

### Windows（WSL 2）
- 要件（公式）：WSLのインストールコマンドは Windows 10 バージョン2004（ビルド19041）以降か Windows 11（[Install WSL](https://learn.microsoft.com/en-us/windows/wsl/install)）。Docker Desktop のWSL 2バックエンドは WSL 2.1.5以降、SLAT対応の64bit CPU、8GBのメモリ、BIOS/UEFIでの仮想化の有効化など（[Docker Desktop on Windows](https://docs.docker.com/desktop/setup/install/windows-install)）。
- 注意：ソースは **WSLのLinux側のファイルシステム** に置くとコンテナのビルドが速い。GUIのソースはWindows側に置く必要があるため、**リポジトリの置き場所が2つに分かれる問題**がある → 推奨：リポジトリはWindows側に1つ。バックエンドのビルドはコンテナのビルド時にコピーする（ボリュームのマウントで速度が出なければ見直し。要実測）。
- WSLg（WSLでLinuxのGUIを動かす機能）はあるが、本番のWindows版の検証にはならないので使わない。
- Docker Desktop のライセンス：個人・教育・非商用OSS・従業員250人未満かつ年間売上1,000万ドル未満の企業は無料、それ以外は有料（[Docker Desktop license](https://docs.docker.com/subscription-billing/desktop-license)）。**事業として使う段階で再確認**。代替は Podman（Apache-2.0）。国内限定では、Docker Desktop 自体の更新確認・診断送信の有無が**要確認**のため、WSL内のPodmanかDocker Engineを推奨にする（10）。

### macOS（Apple container）
- 要件（公式）：Apple siliconのMacが必要で、サポートは macOS 26（それより前は非サポート）。OCI互換のイメージを使い、コンテナごとに軽量VMで動く（[apple/container](https://github.com/apple/container)）。
- 既知の制約（公式）：macOS 26の vmnet の不具合で、補助プログラムが `Documents` や `Desktop` の下にあるとネットワーク作成に失敗する（[BUILDING.md](https://github.com/apple/container/blob/main/BUILDING.md)）。作業フォルダは `~/projects` などに置く。
- 要確認：Docker Compose 相当の複数コンテナの起動の仕組み。ない場合は、起動用のスクリプトかRust製の小さな起動ツールで代替する。
- Intel Mac や macOS 15 以前：Apple container は使えないので Podman か Docker Desktop。
- 「コンテナごとにVM」なので、サンドボックスの隔離はLinuxやWindowsより強いが、起動時間とメモリは要実測（07）。

### Linux
- GUI：X11とWaylandの両方。IMEは fcitx5 と IBus。
- コンテナ：rootlessの Podman を推奨（サンドボックスをrootなしで動かしやすいため）。

## 3. コンテナの構成（ローカル）
`postgres`、`pelway-server`、`pelway-runner`、`tool-gateway`、`llm-stub`、`llm-local`（ローカル推論・任意）、`mail-dummy`、サンドボックス用のイメージ。実行時はどのネットワークも外への出口なし（10）。ネットワークは2つ：`app`（server・DB・runner・gateway）と `sandbox`（サンドボックスとgatewayだけ。外部への出口なし）。

## 4. テスト
| 種類 | どこで | 内容 |
|---|---|---|
| 単体 | コンテナ / ホスト | `cargo test`。状態遷移、承認ハッシュ、冪等キー |
| 結合 | コンテナ | server＋runner＋gateway＋スタブLLM＋ダミーメール。中期・短期の安全試験（応答断20回、再承認20回、権限取消5回/条件）をここで |
| GUI | 各ホスト | gpui-component のヘッドレス試験（公式がうたう機能）＋手作業のIME・スクリーンリーダー試験 |
| Web | ホストのブラウザ | Playwrightなどでの画面試験（ローカル） |
| 性能 | 各ホスト | 07の手順 |

## 5. 配布（設計のみ・今回はしない）
| OS | 形式 | ホストが必要な工程 | 費用・登録 |
|---|---|---|---|
| Windows | MSIかMSIX | Windows上でのビルドと署名 | コード署名証明書が必要（有料・未承認） |
| macOS | `.app` を `.dmg` で | macOS上でのビルド、署名、公証 | Apple Developer Program への登録が必要（有料・未承認） |
| Linux | AppImage / Flatpak / `.deb` | Linux上でのビルド | なし |
- 署名なしでもローカルでは動く（OSの警告が出る）。配布と署名の開始は未決。
