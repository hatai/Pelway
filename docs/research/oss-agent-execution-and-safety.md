# Pelway 将来技術スキャン：AIエージェントの実行と安全

## 結論

MVP前に検証する価値があるのは、すでに仕様の主経路上にある3点です。llama.cppサーバーで動かす商用利用可の日本語モデル、日本語埋め込みモデル ruri-v3、MCP公式Rust SDKの rmcp です。隔離強化の第一候補は crun + libkrun（Podmanの `--runtime krun` で動くmicroVM）です。資格情報ブローカーの第一候補は OpenBao です。ただし両者は仕様の「将来比較」枠のまま扱います。認可エンジン（Cedar 等）は ADR 0004 が MVPでルール言語を入れないと決めているため、MVP後の候補です。

## この文書の前提と読み方

- 調査日は 2026-11-11 です。GitHubのスター数と最終push日はこの日に `gh repo view` と `gh api` で取得しました。数値は取得時点の値です。
- 判定は3種類です。「MVP前に検証」はMVP実装の前に合成データで検証する価値があるものです。「将来の候補」はMVP後の比較対象として残すものです。「見送り」は現時点の制約に合わないものです。
- 「未確認」は、一次情報で確かめられなかった事項に付けています。
- Pelway仕様の参照先は `docs/technical/v0.7/body/` 配下のファイル番号と節名で示します（例：03「隔離と秘密」）。ADRはブランチ `docs/adr-architecture-review` の `docs/adr/0001`〜`0004` です。
- 何もインストール・実行していません。性能・動作は全て未検証です。

### 判定に効くPelway側の制約（仕様から）

- 隔離：sandboxは非root、capabilityなし、read-onlyルート、task用書込領域のみです。microVM / gVisor等は「将来比較」です［Pelway 03 (2026/11), 隔離と秘密］。A6も「microVM等は将来の隔離比較」としています［Pelway 08 (2026/11), A6］。
- 実行ホスト：Linuxはrootless Podman、WindowsはWSL 2内のPodman、macOSはApple containerです［Pelway 05 (2026/11), OS別の境界］。したがってKVM（Linuxカーネルの仮想化機能）が使えるかはホストごとに異なります。WSL 2内でKVMが使えるかは未確認です。
- 推論：実推論候補は国内ホストのllama.cppと事前取得モデルです。起動時のダウンロードと国外fallbackを禁じます［Pelway 03 (2026/11), 監査とローカル推論］。
- 秘密：gatewayの資格情報brokerが、短命でtask/scopeを限定したtokenを出します［Pelway 03 (2026/11), 隔離と秘密］。外部連携停止はgrant epochを増やし、broker tokenと新dispatchを拒否します［Pelway 10 (2026/11), 通信policyを二つに分ける］。
- 認可：AIは「依頼者が委ねた権限」と「役割の権限」の重なりだけで動きます［ADR 0001 (2026/11)］。操作段階は連携ごとの宣言と管理者の確定で決め、MVPではCELのようなルール言語を入れません［ADR 0004 (2026/11)］。
- 出力：AIの実行は依頼チャンネルの読者に結び付け、より広い範囲への投稿は承認を要します［ADR 0003 (2026/11)］。
- MCP：MCPはツール呼出し方式の一つで、ツール説明はデータとして扱い管理者の許可を拡張しません［Pelway 03 (2026/11), 隔離と秘密］。rmcpは条件付き依存候補です［Pelway 11 (2026/11), 接続手段の選び方］［Pelway 09 (2026/11), rmcp行］。

## 1. サンドボックス・隔離ランタイム

Pelwayが解きたい問題は、task使い捨てコンテナがホストカーネルを共有するという隔離限界です［Pelway 08 (2026/11), A6］。

| 候補 | 何か | 適合 | ライセンス・保守 | 判定 |
|---|---|---|---|---|
| crun + libkrun | crunがOCIコンテナを準備した後、libkrunでmicroVMを起動するOCIランタイム | Podmanで `--runtime krun` を指定するだけで使える | libkrun: Apache-2.0、2026-11-10 push | 将来の候補（A6比較の第一候補） |
| gVisor (runsc) | ユーザー空間で動くアプリケーションカーネル | Podmanのrootless経路は文書化あり。ただしネットワーク制約あり | Apache-2.0、2026-11-11 push | 将来の候補 |
| microsandbox | libkrun系のローカルmicroVMランタイム兼ライブラリ | Rust SDKとネットワーク許可リストあり。beta | Apache-2.0、2026-11-11 push | 将来の候補 |
| Landlock (rust-landlock) | Linux LSMのLandlockを使うRustライブラリ | コンテナ内プロセスの追加制限に使える | Apache-2.0 / MIT、2026-11-09 push | 将来の候補 |
| Kata Containers | VMで包むコンテナランタイム | rootlessはVMMだけ。shimはroot | Apache-2.0 | 見送り |
| Firecracker / Cloud Hypervisor | VMM（仮想マシンモニタ）単体 | Podman連携は別途作り込みが要る | Firecracker Apache-2.0。Cloud HypervisorはSPDX取得不可で未確認 | 見送り（単体採用） |
| Bubblewrap | Flatpak等が使う非特権サンドボックスツール | Podmanと役割が重複 | LGPL-2.1 | 見送り |

### crun + libkrun

- 事実：crunはまず通常のOCIコンテナ（名前空間、ルートFS、cgroup制限）を作り、その後libkrunを呼んでmicroVMを起動します［crun krun.1 (2026/11)］。
- 事実：`podman run --runtime krun alpine uname -a` でホストと異なるカーネル版が出ればmicroVMで動いている、と文書は説明しています［crun krun.1 (2026/11)］。
- 事実：前提条件はKVMがロードされていることと、BIOS/UEFIで仮想化支援が有効なことです［crun krun.1 (2026/11)］。
- 事実：ネットワークは既定のTSI（ゲストのソケット呼出しをVMM側で代理）、passt、TAPの3方式です［crun krun.1 (2026/11)］。
- 事実：rootlessでの動作はこの文書では明記されていません。未確認です。
- Pelwayへの当てはめ（筆者の整理）：Podmanのcontroller・イメージ・volumeの扱いを変えずにカーネル共有を解消できる点で、03「隔離と秘密」とA6の比較対象として最も移行費用が小さいと推測します。egress拒否をTSIで保てるかは未確認です。WSL 2とApple containerでは使えない可能性が高く、Linuxホスト限定の選択肢になると推測します。

### gVisor

- 事実：rootless構成では、Podmanのようにエンジン側がユーザー名前空間を用意して `runsc` に渡す方式が使われます［gVisor rootless (2026/11)］。
- 事実：`--rootless` フラグ方式では `create` コマンドが使えず、Netstack（gVisor内蔵のネットワークスタック）が使えないためホストネットワークが必要です［gVisor rootless (2026/11)］。
- 事実：「ネイティブrootless」方式はネットワーク名前空間に未対応です［gVisor rootless (2026/11)］。
- 未確認：Podmanがユーザー名前空間を用意する方式で、ネットワーク名前空間とNetstackが使えるかは、上の文書だけでは確定できません。Podmanの `--userns=keep-id` が未対応とする報告があります［gVisor issue 9918 (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：KVM不要なので、WSL 2などKVMが使えないホストでの代替になり得ます。ただし10「場所・経路の台帳」の「sandboxはgateway以外拒否」を満たすにはネットワーク名前空間が必須なので、ネットワーク方式の検証が採否を決めます。

### microsandbox

- 事実：「local-first microVM runtime and library」で、LinuxではKVMが必要です。READMEは自身を「beta software」と書き、破壊的変更を予告しています［microsandbox README (2026/11)］。
- 事実：設定でネットワークの許可ホスト・許可ポートを指定でき、秘密を特定ホスト宛てに限定する例があります。`cargo add microsandbox` でRust SDKを使えます［microsandbox README (2026/11)］。
- 事実：謝辞にlibkrunとsmoltcpが挙がっています。libkrunを内部で使うかは本文で明記されておらず未確認です［microsandbox README (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：Rust製controllerから直接microVMを扱えるのは魅力です。しかしPodmanを置き換える形になり、05の3OS方針と衝突します。betaのため将来の候補にとどめます。

### Landlock

- 事実：rust-landlockはLinuxのLandlockサンドボックス機能のRustライブラリです。crates.ioの `landlock` 0.4.7 は2026-07-27更新です［rust-landlock repo (2026/11)］［crates.io landlock (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：コンテナ内のrunnerやAdapter子プロセスに、ファイルアクセスの追加制限を自ら課す多層防御として使えます。03は「ホストOS別sandbox」を初版で採らないとしていますが、これはコンテナ内の追加制限なのでその判断とは別物と整理します。カーネル版への依存度は未確認です。

### 見送りの理由

- Kata Containers：rootless VMMモードでも「Only the VMM process drops privileges. The Kata shim and `virtiofsd` still run as `root`」とあります［Kata rootless VMM (2026/11)］。rootless Podman前提の05と合いません。
- Firecracker / Cloud Hypervisor：VMM単体で、OCIイメージの取り扱いやネットワークをPelway側で作り込む必要があります。libkrun経路で同じ目的（KVM microVM）に届くため、単体採用は見送ります。Cloud HypervisorのライセンスはGitHub APIでSPDXを取得できず未確認です［cloud-hypervisor repo (2026/11)］。
- Bubblewrap：LGPL-2.1で、Flatpak向けの非特権サンドボックスツールです［bubblewrap repo (2026/11)］。Podmanがすでに名前空間隔離を提供するため役割が重複します。

## 2. ローカルLLMサービングと日本語モデル

本体AIは提供元のAPIを使う方針になり、この節のローカル推論はMVP前の検証の対象から外れました（[ADR 0008](../adr/0008-llm-provider-and-byok.md)）。一般提供で国内完結の根拠が揃った提供元を選ぶときの候補として残します。

Pelwayが解きたい問題は、A7の「国内hostのlocal inference」をモデルlicense・品質・機材の条件付きで成立させることです［Pelway 08 (2026/11), A7］。

### サービング

| 候補 | 何か | 適合 | ライセンス・保守 | 判定 |
|---|---|---|---|---|
| llama.cpp server | C/C++推論エンジンのHTTPサーバー | 仕様の第一候補。JSON schema制約・埋め込み・rerank・APIキーあり | MIT、2026-11-11 push | MVP前に検証 |
| vLLM | 高スループット推論サーバー | GPUサーバーで同時利用が増えた時の候補 | Apache-2.0、2026-11-11 push | 将来の候補 |
| SGLang | 高性能推論サーバー | vLLMと同じ位置づけ | Apache-2.0、2026-11-11 push | 将来の候補 |
| Ollama | ローカルLLM実行ツール | モデル取得の仕組みが起動時ダウンロード禁止と衝突しないか未確認 | MIT | 見送り |

- llama.cpp：OpenAI互換のchat/embeddings、`/reranking`、grammarまたはJSON schemaによる出力制約、`--parallel` による並列スロット、`--api-key`/`--api-key-file` による認証を備えます。組込みツールとMCP連携は実験的で、明示的に有効化が必要です［llama.cpp server README (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：JSON schema制約は、モデル出力を型付きのActionSnapshot候補へ固定する用途に使えます。組込みツール・MCP連携は無効のまま運用し、ツール実行はgateway経由に限定するのが03「隔離と秘密」と整合します。
- vLLM：使用統計の送信があり、仕様09はその文書を出典に挙げています［Pelway 09 (2026/11), local inference行］［vLLM usage stats (2026/11)］。採用時はこの通信を無効化できるかを10の通信台帳で確認する必要があります。
- Ollama：MIT・Go製です［ollama repo (2026/11)］。実行時にモデルを取得する挙動が、03の「起動時のダウンロード禁止」とどう両立するかは未確認です。内部でllama.cpp系エンジンを使うかも本調査では未確認です。llama.cppを直接使えば足りるため見送ります。

### 日本語対応・商用利用可の生成モデル

Hugging Faceのlicenseタグは提供者の自己申告です。採用時はモデルカードと同梱LICENSE全文を確認する必要があります。

| モデル | ライセンス表示 | 備考 | 判定 |
|---|---|---|---|
| LLM-jp-4 / 4.1（8B、32B-A3B、33B） | apache-2.0 | 公式GGUF版あり（例：llm-jp-4-33b-thinking-gguf） | MVP前に検証 |
| Qwen3-Swallow-8B v0.2 | apache-2.0 | 東工大系。Qwen3ベース | MVP前に検証 |
| Qwen3-8B | apache-2.0 | 「100+ languages and dialects」。日本語の明記はなし | MVP前に検証（比較基準） |
| Sarashina2.2-3B-instruct | mit | 小型 | 将来の候補 |
| ELYZA-Thinking-1.0（llm-jp-4ベース） | apache-2.0 | LLM-jp-4の派生 | 将来の候補 |
| Llama-3.x-Swallow、Llama-3-ELYZA-JP | llama3.x / gemma | Llama・Gemma利用規約の確認が必要 | 見送り |
| PLaMo 3 系 | other | 独自ライセンス。条文未確認 | 見送り（条文確認まで） |

出典：［HF llm-jp (2026/11)］［HF tokyotech-llm (2026/11)］［HF Qwen3-8B (2026/11)］［HF sbintuitions (2026/11)］［HF elyza (2026/11)］［HF pfnet (2026/11)］。

- Pelwayへの当てはめ（筆者の整理）：A7は「モデル名・hash・ライセンスと量子化条件を固定」と求めています。Apache-2.0/MITのモデルは、Llama/Gemma系の利用規約の条件を確認する手間を省けます。日本語品質・GPU/RAM要件は未測定です。

### 日本語埋め込みモデル

| モデル | ライセンス | 要点 | 判定 |
|---|---|---|---|
| ruri-v3（30m〜310m）と ruri-v3-reranker | Apache-2.0 | ModernBERT-Ja基盤、最大8192トークン、310mのJMTEB平均77.24 | MVP前に検証 |
| PLaMo-Embedding-1B | apache-2.0（タグ） | 1Bパラメータ | 将来の候補 |
| multilingual-e5-large | MIT | xlm-roberta-large基盤、100言語 | 将来の候補（比較基準） |

出典：［HF ruri-v3-310m (2026/11)］［HF cl-nagoya (2026/11)］［HF pfnet (2026/11)］［HF multilingual-e5-large (2026/11)］。

- 未確認：ruri-v3（ModernBERT系）がllama.cppのGGUF形式で動くかは確認できていません。動かない場合、埋め込み用に別の推論プロセスが要り、03の「llama.cppと事前取得モデル」の範囲を広げる判断が必要になります。
- Pelwayへの当てはめ（筆者の整理）：ADR 0003は「記憶の検索は、今の実行の範囲より広いところから得た記憶を返さない」としています。埋め込み検索はチャンネル範囲で事前フィルタする設計と組み合わせる必要があります。

## 3. 認可・ポリシーエンジン

Pelwayが解きたい問題は、委任∩役割の権限（ADR 0001）、チャンネル単位の読者範囲（ADR 0003）、承認者指定（ADR 0002）の判定です。ADR 0004はMVPでルール言語を入れないと決めています。そのためここでの候補は全てMVP後の扱いです。

| 候補 | 何か | 適合 | ライセンス・保守 | 判定 |
|---|---|---|---|---|
| Cedar（`cedar-policy` crate） | Rust実装のポリシー言語。スキーマ検証と自動推論による解析が可能 | Rustプロセス内で評価でき、別ストア不要 | Apache-2.0、crate 4.14.1（2026-11-02） | 将来の候補 |
| OpenFGA | Zanzibar型の関係ベース認可サーバー。CNCF incubating | 別サーバー・別ストア。Rustクライアントは第三者製 | Apache-2.0、Go | 見送り |
| SpiceDB | Zanzibar型の認可データベース | 同上 | Apache-2.0、Go | 見送り |
| Casbin-rs | ACL/RBAC/ABACのRustライブラリ | Rustで使えるが関係・委任の表現はCedarより弱いと推測 | Apache-2.0、crate 2.20.0（2026-02-04） | 見送り |
| OPA / Rego | 汎用ポリシーエンジン | Go製サーバー。Rust組込みは未確認 | Apache-2.0 | 見送り |
| Oso | 旧OSSライブラリ | リポジトリはarchived | — | 見送り |

- Cedar：「designed for analysis using Automated Reasoning」で、スキーマに対する検証器と、反例付きで性質を検証する `cedar-policy-symcc` があります［cedar repo (2026/11)］［crates.io cedar-policy (2026/11)］。Lean による形式検証やCNCFでの位置づけは、今回取得したページには記載がなく未確認です。
- Pelwayへの当てはめ（筆者の整理）：「AIはprincipal、依頼者の委任と役割の上限をcontextとして渡す」形で、ADR 0001の重なり判定を宣言的に書けます。PostgreSQLを正本とする方針（A3）を崩さずに、ポリシー文字列をDBに版管理して保持できます。ただしADR 0004と衝突するため、導入するならADRの改訂が先です。
- OpenFGA / SpiceDB：OpenFGAはCNCF incubatingでZanzibar（Googleの認可システム論文）由来です［openfga.dev (2026/11)］。関係タプルを別ストアに持つため、PostgreSQLを唯一の正本とするA3と二重管理になります。Rustクライアントは `openfga-rs` 0.1.0（2024-04-08）と `spicedb-rust` 0.3.4（2024-12-01）で、いずれも第三者製かつ更新が止まっています［crates.io openfga-rs (2026/11)］［crates.io spicedb-rust (2026/11)］。
- Oso：「Legacy Oso open source library (archived)」です［oso repo (2026/11)］。

## 4. 秘密ブローカー

Pelwayが解きたい問題は、gatewayの資格情報brokerが短命でscope限定のtokenを出し、連携停止時にまとめて失効させることです［Pelway 03 (2026/11), 隔離と秘密］［Pelway 10 (2026/11), 通信policyを二つに分ける］。

| 候補 | 何か | 適合 | ライセンス・保守 | 判定 |
|---|---|---|---|---|
| OpenBao | Vaultのフォーク。Linux Foundation OpenSSF配下 | リース・TTL・prefix単位の失効がgrant epochと対応 | MPL-2.0、2026-11-10 push | 将来の候補 |
| vaultrs | Vault APIの非同期Rustクライアント | OpenBao対応は議論中 | MIT、crate 0.8.0（2026-03-17） | 将来の候補（OpenBaoと組で） |
| HashiCorp Vault | 秘密管理製品 | BSLのため見送り | Business Source License | 見送り |
| Infisical | 秘密管理プラットフォーム | TypeScript製。`ee/` 配下は別ライセンス | MIT＋ee別ライセンス | 見送り |

- OpenBao：OpenSSFのSandboxプロジェクトとして運営されています［openbao.org (2026/11)］。動的秘密には必ずリースがあり、期限切れで自動失効し、tokenを失効させるとそのtokenで作った全リースが失効します。リースIDの先頭は取得パスなので、パス単位でまとめて失効できます［OpenBao lease (2026/11)］。
- vaultrs：OpenBao対応を求めるissueでは、挙動の差を1件見つけたことと、VaultとOpenBaoの両方で試験する案が議論されています。正式対応は未確認です［vaultrs issue 150 (2026/11)］。
- Vault：LICENSE本文がBusiness Source Licenseです［vault LICENSE (2026/11)］。
- Infisical：「All content that resides under any "ee/" directory ... are licensed under the license defined in "ee/LICENSE"」とあります［infisical LICENSE (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：「連携ごとに1パス、連携停止＝パスprefixの失効」と設計すると、10のgrant epochを外部の秘密も含めて強制できます。一方で、OpenBao自体の封印解除鍵の管理や国内バックアップが増えます。MVPではgateway内の自前brokerで足りる可能性が高く、将来の候補とします。

## 5. MCP基盤とエージェント間プロトコル

| 候補 | 何か | 適合 | ライセンス・保守 | 判定 |
|---|---|---|---|---|
| rmcp | MCP公式Rust SDK | 仕様で条件付き依存候補 | MIT→Apache-2.0移行中、crate 3.9.0（2026-11-05） | MVP前に検証 |
| MCPツールannotations | `readOnlyHint` 等のヒント | 仕様自体が「untrusted」と明記 | — | MVP前に検証（ADR 0004の扱いの確認） |
| agentgateway | MCP/A2A向けプロキシ。Linux Foundation | Rust系。CEL・RBAC。承認機能の記載なし | Apache-2.0、2026-11-11 push | 将来の候補（設計参照） |
| IBM mcp-context-forge | MCP/A2A/RESTのゲートウェイ | Python/FastAPI。RC版 | Apache-2.0 | 見送り |
| microsoft/mcp-gateway | MCPのリバースプロキシ | C# | MIT | 見送り |
| A2A | エージェント間のtask委任プロトコル。Linux Foundation | 仕様11で参考候補 | Apache-2.0 | 将来の候補 |
| ACP（Agent Client Protocol） | エディタとコーディングエージェントの通信規約 | 外部コーディングagent連携に使える可能性 | Apache-2.0、crate 5.2.0（2026-11-10） | 将来の候補 |
| AG-UI | エージェントとフロントエンドの対話プロトコル | Pelwayは独自UI・独自同期 | MIT、TypeScript | 見送り |

- rmcp：LICENSEは「MITからApache-2.0へ移行中、再許諾の同意がない旧寄稿はMITのまま」で、仕様09がすでにこの点を記録しています［rust-sdk LICENSE (2026/11)］［Pelway 09 (2026/11), rmcp行］。MVP前の検証項目は、固定版のMIT部分の特定（U05）と、gateway経由以外の通信を持たない構成で使えるかです。
- annotations：MCP仕様（2025-06-18版）は「clients **MUST** consider tool annotations to be untrusted unless they come from trusted servers」と定めます。また「there **SHOULD** always be a human in the loop with the ability to deny tool invocations」としています［MCP tools 2025-06-18 (2026/11)］。最新版の仕様文言は未確認です。
- Pelwayへの当てはめ（筆者の整理）：ADR 0004の「annotationsは手がかりとして表示し、段階は管理者が確定」は、この仕様文言と一致します。
- agentgateway：「Agentgateway is a Linux Foundation project」で、MCPツールのフェデレーション（stdio/HTTP/SSE/Streamable HTTP）、A2A、「fine-grained RBAC with CEL policy engine」を備えます。ツール許可リストと承認（human-in-the-loop）はページに記載がありません［agentgateway README (2026/11)］。
- Pelwayへの当てはめ（筆者の整理）：Pelwayのgatewayは承認・ActionSnapshot hash・監査を一体で持つ必要があり、外部プロキシに置き換えると責任分界が増えます。製品として採るより、MCP中継の実装を参照する対象とします。
- IBM mcp-context-forge：Python ≥ 3.11・FastAPI製で、版は「1.0.0-RC-3」のようなRCタグです。クロスゲートウェイのドメイン許可リストは空なら全拒否です。承認機能の記載はページにありません［mcp-context-forge README (2026/11)］。Rust/単一バックエンドの方針と合わないため見送ります。
- A2A：「an open source project under the Linux Foundation, contributed by Google」で、SDK一覧にRustが含まれます。Rust SDKが公式扱いかは未確認です。crates.ioの `a2a-rs` は第三者製です［A2A README (2026/11)］［crates.io a2a-rs (2026/11)］。
- ACP：エディタ/IDEとコーディングエージェントの通信を標準化し、ローカルでは子プロセスのstdin/stdout上のJSON-RPCを使います［agentclientprotocol.com (2026/11)］。Rust crateは `agent-client-protocol` 5.2.0 です［crates.io agent-client-protocol (2026/11)］。作成主体と権限要求の仕組みは今回のページでは確認できず未確認です。
- Pelwayへの当てはめ（筆者の整理）：11「外部agentの権限を過信しない」で扱う外部コーディングagentを、CLI出力の解析ではなく型付きプロトコルで包める可能性があります。
- AG-UI：MIT・TypeScriptです［ag-ui repo (2026/11)］。PelwayはSlint/Octaneと独自の同期を持つため（A1、A2、A4）、利点が見えず見送ります。プロトコルの中身は未確認です。

## 6. 情報フロー制御とプロンプトインジェクション対策

| 候補 | 何か | 適合 | 状態 | 判定 |
|---|---|---|---|---|
| CaMeL | 制御フローとデータフローを分離し、capabilityでツール呼出しを制限 | ADR 0003の設計根拠を補強 | 研究実装。repo最終push 2025-06-20 | 将来の候補（設計参照） |
| FIDES | 機密性・完全性ラベルによる情報フロー制御のプランナー | ADR 0003の「範囲ラベル」と同型 | 研究実装。repo最終push 2025-05-30 | 将来の候補（設計参照） |
| Guardrails AI / NeMo Guardrails | LLM入出力の検査ツールキット | Python製。決定的保証ではない（推測） | Apache-2.0 | 見送り |

- CaMeL：信頼できる利用者クエリから制御フローを取り出し、取得した非信頼データがプログラムの流れを変えられないようにします。capabilityでツール呼出し時のポリシーを強制します。AgentDojoで「provable security」付きで77%のタスクを解き、無防御では84%でした［CaMeL arXiv (2025/03)］。
- FIDES：データに機密性と完全性のラベルを付け、ラベルにもとづくポリシーを決定的に強制し、情報を選択的に隠すプリミティブを持ちます。AgentDojoで評価しています［FIDES arXiv (2025/05)］。
- Pelwayへの当てはめ（筆者の整理）：ADR 0003の「記憶に範囲を付ける」「投稿直前に範囲を確かめる」は、FIDESの機密性ラベルを読者集合で表したものと読めます。コードは研究用ノートブックが中心のため、ライブラリとして取り込まず設計の参照にとどめます。
- Guardrails系：Guardrails AIとNeMo GuardrailsはどちらもPython製です［guardrails repo (2026/11)］［NeMo Guardrails repo (2026/11)］。検査がモデル判定に依存する範囲は未確認です。ADR 0003は「モデルに漏らさないことを教える方法には頼れない」としているので、主防御には使いません。

## 上位5件の推奨

1. llama.cpp server と Apache-2.0の日本語モデル（LLM-jp-4、Qwen3-Swallow、Qwen3）：MVP前に検証。仕様の第一候補で、JSON schema制約と並列スロットがあり、ライセンスが明快です。
2. ruri-v3（埋め込み・reranker）：MVP前に検証。Apache-2.0で日本語特化です。ただしllama.cppで動くかは未確認で、動かなければ推論プロセスの範囲判断が要ります。
3. rmcp とMCP annotationsの非信頼扱い：MVP前に検証。仕様上の条件付き依存で、MIT部分の特定（U05）とADR 0004の扱いが公式仕様と整合することを固定版で確かめます。
4. crun + libkrun：将来の候補（A6の第一比較対象）。Podmanのまま `--runtime krun` でmicroVM化でき、移行費用が最小と推測します。KVM必須で、rootlessとegress拒否は未確認です。
5. OpenBao（＋vaultrs）：将来の候補。リースとprefix失効がgrant epochに対応します。ただし運用負荷が増え、vaultrsのOpenBao対応は未確定です。

次点：Cedar（ADR 0004改訂後の認可エンジン候補）、FIDES/CaMeL（ADR 0003の設計参照）。

## 出典

[Pelway 03, 2026/11] Pelway. 「03 Bot実行・権限・承認・隔離」. docs/technical/v0.7/body/03-ai-bot-runtime.md
[Pelway 05, 2026/11] Pelway. 「05 3OSのローカル開発・試験・配布設計」. docs/technical/v0.7/body/05-local-dev.md
[Pelway 08, 2026/11] Pelway. 「08 採用記録・未決の正本・受入試験」. docs/technical/v0.7/body/08-decisions-open.md
[Pelway 09, 2026/11] Pelway. 「09 出典」. docs/technical/v0.7/body/09-sources.md
[Pelway 10, 2026/11] Pelway. 「10 Pelway本体の国内限定と外部連携の責任分界」. docs/technical/v0.7/body/10-domestic-only.md
[Pelway 11, 2026/11] Pelway. 「11 AIネイティブ連携の共通契約」. docs/technical/v0.7/body/11-integrations.md
[ADR 0001–0004, 2026/11] Pelway. docs/adr/0001〜0004（ブランチ docs/adr-architecture-review）
[crun krun.1, 2026/11] containers. "krun.1." https://github.com/containers/crun/blob/main/krun.1.md
[libkrun repo, 2026/11] libkrun. "libkrun." https://github.com/libkrun/libkrun
[gVisor rootless, 2026/11] gVisor. "Rootless." https://gvisor.dev/docs/user_guide/rootless/
[gVisor issue 9918, 2026/11] google/gvisor. Issue 9918. https://github.com/google/gvisor/issues/9918
[gvisor repo, 2026/11] google. "gvisor." https://github.com/google/gvisor
[microsandbox README, 2026/11] superradcompany. "microsandbox." https://github.com/superradcompany/microsandbox
[rust-landlock repo, 2026/11] landlock-lsm. "rust-landlock." https://github.com/landlock-lsm/rust-landlock
[crates.io landlock, 2026/11] crates.io. "landlock." https://crates.io/crates/landlock
[Kata rootless VMM, 2026/11] kata-containers. "How to run a rootless VMM." https://github.com/kata-containers/kata-containers/blob/main/docs/how-to/how-to-run-rootless-vmm.md
[firecracker repo, 2026/11] firecracker-microvm. "firecracker." https://github.com/firecracker-microvm/firecracker
[cloud-hypervisor repo, 2026/11] cloud-hypervisor. "cloud-hypervisor." https://github.com/cloud-hypervisor/cloud-hypervisor
[bubblewrap repo, 2026/11] containers. "bubblewrap." https://github.com/containers/bubblewrap
[llama.cpp server README, 2026/11] ggml-org. "llama.cpp server." https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
[vLLM usage stats, 2026/11] vLLM. "Usage Stats Collection." https://docs.vllm.ai/en/stable/usage/usage_stats/
[vllm repo, 2026/11] vllm-project. "vllm." https://github.com/vllm-project/vllm
[sglang repo, 2026/11] sgl-project. "sglang." https://github.com/sgl-project/sglang
[ollama repo, 2026/11] ollama. "ollama." https://github.com/ollama/ollama
[HF llm-jp, 2026/11] Hugging Face. llm-jp models. https://huggingface.co/llm-jp
[HF tokyotech-llm, 2026/11] Hugging Face. tokyotech-llm models. https://huggingface.co/tokyotech-llm
[HF Qwen3-8B, 2026/11] Qwen. "Qwen3-8B." https://huggingface.co/Qwen/Qwen3-8B
[HF sbintuitions, 2026/11] Hugging Face. sbintuitions models. https://huggingface.co/sbintuitions
[HF elyza, 2026/11] Hugging Face. elyza models. https://huggingface.co/elyza
[HF pfnet, 2026/11] Hugging Face. pfnet models. https://huggingface.co/pfnet
[HF cl-nagoya, 2026/11] Hugging Face. cl-nagoya models. https://huggingface.co/cl-nagoya
[HF ruri-v3-310m, 2026/11] cl-nagoya. "ruri-v3-310m." https://huggingface.co/cl-nagoya/ruri-v3-310m
[HF multilingual-e5-large, 2026/11] intfloat. "multilingual-e5-large." https://huggingface.co/intfloat/multilingual-e5-large
[cedar repo, 2026/11] cedar-policy. "cedar." https://github.com/cedar-policy/cedar
[crates.io cedar-policy, 2026/11] crates.io. "cedar-policy." https://crates.io/crates/cedar-policy
[openfga.dev, 2026/11] OpenFGA. https://openfga.dev/
[spicedb repo, 2026/11] authzed. "spicedb." https://github.com/authzed/spicedb
[crates.io openfga-rs, 2026/11] crates.io. "openfga-rs." https://crates.io/crates/openfga-rs
[crates.io spicedb-rust, 2026/11] crates.io. "spicedb-rust." https://crates.io/crates/spicedb-rust
[casbin-rs repo, 2026/11] apache. "casbin-rs." https://github.com/apache/casbin-rs
[opa repo, 2026/11] open-policy-agent. "opa." https://github.com/open-policy-agent/opa
[oso repo, 2026/11] osohq. "oso." https://github.com/osohq/oso
[openbao.org, 2026/11] OpenBao. https://openbao.org/
[OpenBao lease, 2026/11] OpenBao. "Lease, Renew, and Revoke." https://openbao.org/docs/concepts/lease/
[vaultrs issue 150, 2026/11] jmgilman/vaultrs. "Openbao support?" https://github.com/jmgilman/vaultrs/issues/150
[vault LICENSE, 2026/11] hashicorp. "LICENSE." https://github.com/hashicorp/vault/blob/main/LICENSE
[infisical LICENSE, 2026/11] Infisical. "LICENSE." https://github.com/Infisical/infisical/blob/main/LICENSE
[rust-sdk LICENSE, 2026/11] modelcontextprotocol. "LICENSE." https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
[MCP tools 2025-06-18, 2026/11] Model Context Protocol. "Tools." https://modelcontextprotocol.io/specification/2025-06-18/server/tools
[agentgateway README, 2026/11] agentgateway. "agentgateway." https://github.com/agentgateway/agentgateway
[mcp-context-forge README, 2026/11] IBM. "mcp-context-forge." https://github.com/IBM/mcp-context-forge
[microsoft mcp-gateway, 2026/11] microsoft. "mcp-gateway." https://github.com/microsoft/mcp-gateway
[A2A README, 2026/11] a2aproject. "A2A." https://github.com/a2aproject/A2A
[crates.io a2a-rs, 2026/11] crates.io. "a2a-rs." https://crates.io/crates/a2a-rs
[agentclientprotocol.com, 2026/11] Agent Client Protocol. https://agentclientprotocol.com/
[crates.io agent-client-protocol, 2026/11] crates.io. "agent-client-protocol." https://crates.io/crates/agent-client-protocol
[ag-ui repo, 2026/11] ag-ui-protocol. "ag-ui." https://github.com/ag-ui-protocol/ag-ui
[CaMeL arXiv, 2025/03] Debenedetti et al. "Defeating Prompt Injections by Design." arXiv:2503.18813. https://arxiv.org/abs/2503.18813
[camel repo, 2026/11] google-research. "camel-prompt-injection." https://github.com/google-research/camel-prompt-injection
[FIDES arXiv, 2025/05] "Securing AI Agents with Information-Flow Control." arXiv:2505.23643. https://arxiv.org/abs/2505.23643
[fides repo, 2026/11] microsoft. "fides." https://github.com/microsoft/fides
[guardrails repo, 2026/11] guardrails-ai. "guardrails." https://github.com/guardrails-ai/guardrails
[NeMo Guardrails repo, 2026/11] NVIDIA-NeMo. "Guardrails." https://github.com/NVIDIA-NeMo/Guardrails
