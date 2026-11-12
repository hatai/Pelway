# Pelway 将来技術調査：プラットフォームサービスとクライアント

- 確認日：2026-11-11。GitHub のスター数・最終 push・ライセンスは `gh repo view` / `gh api repos/<r>/license` で同日に取得した値。以下「GH (2026/11)」と表記する。
- 範囲：調査のみ。インストール、実行、リポジトリの編集はしていない。
- 判定の区分：「MVP前に検証」は v0.7 の未決（U05）または採用済み選定（A1/A2）の検証項目に直結するものに限った。仕様が初期不使用・将来扱いと決めている領域（APNs/FCM、OpenTelemetry、E2EE、Matrix）は、仕様どおり「将来の候補」か「見送り」とした。
- 各節は「事実（出典つき）」と「Pelwayへの当てはめ（筆者の判断）」に分けた。当てはめは筆者の解釈で、出典元の主張ではない。
- 未確認の事項は「未確認」と明記した。

## 0. 前提にした Pelway 仕様

- U05 は「server認証/Origin/TLS/service identity」「暗号鍵/連携token分離」などを未決として挙げ、「新サービスを自動追加しない」としている [Pelway v0.7 08 (2026/11), U05]。
- 運営アクセスの章は「外部の権限管理サービスを必須にしない」とし、E2EEは一律必須としない。serverと国内runnerが平文を扱う信頼モデルを明示する、としている [Pelway v0.7 12 (2026/11), §7]。
- 通知は framework 外の host adapter によるローカル通知。「APNs/FCMは初期不使用」[Pelway v0.7 02 (2026/11), チャット要件での評価]。
- 観測は tracing のローカル出力。「OpenTelemetryは将来の相互運用候補。外部exporterや自動設定を初期有効にしない」[Pelway v0.7 02 (2026/11), サーバー・AI層の比較]。
- Matrix と Zulip は同期設計の参考で、「server/UIのコードを採用しない」[Pelway v0.7 09 (2026/11), 同期]。
- A1 は Slint で、検証項目に IME、NVDA/VoiceOver/Orca、大量可変高リストを挙げる。A2 は Web を TS/React DOM、同期を Rust/WASM とし、検証項目に「WASM起動/サイズと型生成維持費」を挙げる [Pelway v0.7 08 (2026/11), A1/A2]。
- 音声・ビデオの要件は v0.7 本文に見当たらなかった（`grep` で該当なし）。

---

## 1. 自前ホストの認証・ID

### 1.1 根本の選択：axum に埋め込むライブラリか、別プロセスの IdP か

**Pelwayへの当てはめ（筆者の判断）**
U05 は「新サービスを自動追加しない」とし、12 §7 は外部の権限管理サービスを必須にしない。このため、初期は axum に WebAuthn（パスキー）と OIDC Relying Party（顧客の Google / Microsoft IdP でのログイン）を組み込む案が制約に最も合う。IdP 製品（Kanidm、Ory など）は、OIDC プロバイダーとして他アプリへ SSO を配る必要が出た段階で再検討する。

### 1.2 webauthn-rs（Rust、パスキー）

- 事実：kanidm/webauthn-rs。MPL-2.0、721 stars、最終 push 2026-11-05 [GH (2026/11)]。Relying Party 向けの高水準で安全な API（`webauthn-rs`）と低水準の `webauthn-rs-core` に分かれる。iCloud Keychain や Google Password Manager のような同期型パスキーでも動く。SUSE product security の監査を通過している [webauthn-rs README (2026/11)]。Kanidm 本体が WebAuthn level 3 を機能として掲げている [Kanidm book (2026/11), supported features]。
- 解決する問題：U05 の server 認証。パスワードを持たないログインと、承認操作の再認証（user verification）。
- 当てはめ：Rust/axum に直接組み込め、追加サービスが要らない。MPL-2.0 はファイル単位の copyleft なので、改変しない依存として使うなら配布物の NOTICE 管理で足りる見込み（法的判断は未確認）。
- 複雑さ：中。資格情報の保存、RP ID/Origin の固定、端末紛失時の復旧手順を自前で設計する必要がある。
- 判定：**MVP前に検証**（U05）。

### 1.3 openidconnect-rs（Rust、OIDC クライアント）

- 事実：ramosbugs/openidconnect-rs。MIT、652 stars、最終 push 2026-10-31 [GH (2026/11)]。型付きの OIDC インターフェースで、Google、GitLab、Microsoft などでの利用者認証に使える。4.1 以降の MSRV は Rust 1.88 [openidconnect-rs README (2026/11)]。
- 解決する問題：顧客の Google Workspace / Microsoft Entra をログイン元にする要件（U05）。
- 当てはめ：Pelway が Relying Party になるだけなら IdP 製品は要らない。テナントごとの issuer 設定と、ドメイン制限（hd / tid クレーム）の検証は自前で書く。
- 判定：**MVP前に検証**（U05）。顧客 IdP 連携を初期範囲に入れるかは未決。

### 1.4 Kanidm（Rust、IdP）

- 事実：MPL-2.0、5,559 stars、最終 push 2026-11-10、最新 v1.12.0（2026-11-01）[GH (2026/11)]。OAuth2/OIDC プロバイダーで、公開クライアントには PKCE S256 が必須、署名は ES256 が必須 [Kanidm OAuth2 (2026/11)]。機能には WebAuthn level 3、読取専用 LDAP、RADIUS があり、DB は `/data/kanidm.db` のようなパスに保存され、ACID を掲げる。PostgreSQL への言及は見当たらなかった（外部 DB を使えないかは未確認）[Kanidm book (2026/11), supported features / server config]。FreeIPA と LDAP からの同期機能がある [Kanidm book src (2026/11), sync/]。
- 未確認：Google / Microsoft を上流 IdP としてログインさせる機能（上流フェデレーション）は、文書の中に記述を見つけられなかった。
- 当てはめ：Rust で軽量だが、PostgreSQL と別の DB ファイルを持つと見られるため、バックアップ、削除、復旧の順序（12 §8）が二系統になる。顧客 IdP 連携が確認できない限り、Pelway の主要件に合わない。
- 判定：**将来の候補**。

### 1.5 Rauthy（Rust、IdP）

- 事実：Apache-2.0、1,443 stars、最終 push 2026-11-11、最新 v0.37.2（2026-10-19）[GH (2026/11)]。DB は組込みの Hiqlite（既定）か PostgreSQL。少数ユーザーなら約 35〜65 MB で動く。パスキー専用アカウントに対応する。Radically Open Security の監査を受けている。サポートは主に作者の空き時間で行われる [Rauthy README (2026/11)]。上流認証プロバイダーは "in beta state" で、設定ガイドは GitHub だけ [Rauthy book (2026/11), auth_providers/index.md]。
- 当てはめ：PostgreSQL を共有でき、資源も小さい。ただし保守は個人中心で、Google / Microsoft の上流連携はベータ扱い。
- 判定：**将来の候補**。

### 1.6 Ory（Kratos / Hydra / Keto）：重点評価

**事実**

- ライセンスと活動：kratos 13,954 stars、hydra 17,659、keto 5,422、oathkeeper 3,611、polis 2,272。いずれも Apache-2.0 [GH (2026/11)]。kratos と hydra の最終 push は 2026-07-29、keto は 2026-09-04 [GH (2026/11)]。kratos の直近のコミットには `GitOrigin-RevId:` 行があった（`gh api repos/ory/kratos/commits` で確認）。これは社内リポジトリから GitHub へ写している形跡に見えるが、Ory 自身の説明は見つけていない（未確認）。Ory のブログは Keto/Oathkeeper の "monorepo migration" に触れている [Ory OSS v25.4.0 (2025/11)]。
- リリース：kratos と hydra の最新 OSS リリースはどちらも v26.2.0（2026-03-20）で、その前は v25.4.0（2025-11-07）[GH (2026/11)]。OSS は暦年版（YY.Q.B）で、OSS 向けには `.0` 版だけを出す。OEL（Ory Enterprise License）では内部リリースごとにパッチ版が出る [Ory OSS v25.4.0 (2025/11)]。
- OSS と OEL の違い：OEL 限定の機能は、Kratos のマルチテナント・B2B 組織、Hydra の ROPC grant、無停止アップグレードと無停止スキーマ移行、CockroachDB での複数リージョン構成など。OSS はコミュニティでのサポートで、ライセンスなしでは更新が "can lag and are not tested at the same scale"。CVE パッチの保証は OEL か Ory Continuous Security License が必要 [Ory OEL docs (2026/11)]。kratos の README も、CVE 修正の保証には OEL と Enterprise Docker registry を案内している [Kratos README (2026/11)]。LTS の有無は未確認。
- OpenAI の事例：Ory の事例ページは、OpenAI が "adopted an Ory Enterprise License for Ory Hydra" と書く。規模の試験は Ory Network と CockroachDB で行った [Ory case study OpenAI (2026/11)]。Ory のブログは "OpenAI's OAuth2 infrastructure runs on Ory Hydra" と書く [Ory blog OpenAI (2026/11)]。事例ページは自前ホストを選定時の要件に挙げているが、本番が自前ホストか Ory Network かは、どちらのページにも書かれていない（未確認）。CockroachDB も試験の文脈で書かれており、本番の DB とは明記されていない。OpenAI の事例は Hydra だけで、Kratos と Keto は含まれない。
- Kratos：パスキー専用の戦略があり、自前ホストでは `selfservice.methods.passkey.enabled` で有効にする [Kratos passkeys (2026/11)]。ソーシャルログインは 15 以上の事業者（Google を含む）と、汎用の OIDC プロバイダーに対応する [Kratos social sign-in (2026/11)]。Microsoft の専用プロバイダーの有無は、このページでは確認できなかった（汎用 OIDC で接続可能かは未確認）。MFA（TOTP など）の OSS 版での提供範囲は確認していない（未確認）。デプロイ例は PostgreSQL を使う [Kratos deploy example (2026/11)]。
- Hydra / Keto の DB：両 README が「PostgreSQL、MySQL、CockroachDB を設定する」と書いている [Hydra README (2026/11)] [Keto README (2026/11)]。
- 日本語 UI：ログイン画面の部品集 ory/elements（Apache-2.0、最終 push 2026-11-10）に `ja.json` ロケールがある [ory/elements locales (2026/11)]。翻訳の品質は未確認。
- AI エージェント向け：Hydra v25.4 は OAuth 2.0 Device Authorization Flow を入れ、Ory はこれを「agentic authentication」と呼ぶ。MCP サーバー認証のガイドもある [Ory blog OAuth for AI agents (2026/11)]。一方、RFC 8693 Token Exchange は Hydra の issue #1218 で open、マイルストーンは "unplanned" [ory/hydra #1218 (2026/11)]。OEL / Ory Network で提供されるかは未確認。MCP クライアント向け CIMD の issue #4134 は duplicate として閉じられた [ory/hydra #4134 (2026/11)]。
- Keto：Google Zanzibar の設計に基づく権限サーバーで、権限言語 OPL は TypeScript のサブセット [Keto docs (2026/11)]。比較対象として、OpenFGA（Apache-2.0、6,131 stars）は memory / Postgres / MySQL / SQLite に対応する [OpenFGA config (2026/11)] [GH (2026/11)]。SpiceDB（Apache-2.0、7,215 stars）は単一リージョンの自前ホストなら PostgreSQL が向くとしている [SpiceDB datastores (2026/11)] [GH (2026/11)]。

**Pelwayへの当てはめ（筆者の判断）**

- 勢いを示す根拠（OpenAI）は、OEL の Hydra と CockroachDB の組み合わせについてのもの。Pelway が使うことになる OSS の Kratos/Hydra とは、パッチ供給の条件が異なる。
- 1 台のホストで rootless Podman を使うなら、Kratos と Hydra（と UI）で 2〜3 個の Go サービスが増える。OSS の CVE 修正は `.0` 版を待つ必要があり、遅れうる。小規模の日本企業向けで運営者が一人なら、この運用負担は大きい。
- Kratos の汎用 OIDC とパスキー、Elements の `ja.json` は Pelway の要件に合う。顧客 IdP の連携を自前で書く量を減らせる。
- Pelway に必要な「AI が承認を経て行う委任」は、正確な操作への承認であり、OAuth のスコープより細かい。Hydra は RFC 8693 を OSS で提供しておらず、委任の中心には使えない。承認の正本は Pelway server と PostgreSQL に置いたままにするのが妥当。
- Keto / OpenFGA / SpiceDB は 12 §7（外部の権限管理サービスを必須にしない）と衝突する。チャンネル単位の ACL は PostgreSQL の表と SQL で表現できる規模にとどまる。
- 判定：Kratos（＋必要なら Hydra）は **将来の候補**。顧客が SSO を Pelway 以外にも配りたい、などの需要が出た時に Kanidm / Rauthy と並べて再評価する。Keto・OpenFGA・SpiceDB は **見送り**（12 §7）。Oathkeeper と Polis は **見送り**。

### 1.7 Zitadel / Keycloak / Authentik

- Zitadel：v3.0 から本体、Console、Hosted Login を AGPL-3.0 に変更し、新規コードから順に適用する。API 定義と SDK は Apache-2.0 のまま [Zitadel AGPL (2025/03)]。15,559 stars [GH (2026/11)]。
- Keycloak：Apache-2.0、37,821 stars [GH (2026/11)]。PostgreSQL 14〜18 に対応する。組込みの `dev-file` DB は本番に向かない [Keycloak DB (2026/11)]。JVM の資源要件は確認していない（未確認）。
- Authentik：本体は MIT、`authentik/enterprise/` は別のライセンス [Authentik LICENSE (2026/11)]。26,307 stars [GH (2026/11)]。Google / Entra を上流ソースにできるという記述は検索結果の見出しで見ただけで、本文は確認していない（未確認）。
- 当てはめ：いずれも追加サービスが重い（Keycloak は JVM、Authentik は Python ＋ worker で構成は未確認）。Zitadel は AGPL への移行で同梱・改変の条件が重くなる。
- 判定：Keycloak は「顧客が既に Keycloak を使っている」場合の接続先として **将来の候補**。Zitadel と Authentik は **見送り**。

---

## 2. エンドツーエンド暗号化（MVP外）

- OpenMLS：MIT、1,065 stars、最新 v0.9.0（2026-08-25）[GH (2026/11)]。MLS（RFC 9420）は「2人から数千人」のグループで、前方秘匿性と侵害後の安全性を持つ非同期の鍵確立を提供する [RFC 9420 (2023/07), Abstract]。
- vodozemac：Apache-2.0、416 stars [GH (2026/11)]。Matrix の Olm/Megolm の Rust 実装で、libolm の代替。Least Authority の監査を受けている [vodozemac README (2026/11)]。
- AI のサーバー処理との衝突（筆者の判断）：E2EE では server と国内 runner が平文を読めない。このため要約、検索索引、AI の役割処理、DLP、運営による本文非閲覧の調査（12 §7）が、すべてクライアント側か「AI を正規のグループ参加者にして鍵を渡す」方式に変わる。後者では AI ホストが平文を持つため、E2EE の保証は「server 運営者からの秘匿」ではなく「AI 実行ホストを信頼する」形に弱まる。12 §7 が E2EE を一律必須とせず平文の信頼モデルを明示しているのは、この衝突を避ける判断と整合する。
- 判定：OpenMLS は **将来の候補**（AI を使わない DM や秘密チャンネル用の選択肢）。vodozemac は Matrix 互換をやる場合だけ意味があり **見送り**。

---

## 3. 音声・ビデオ（ハドル）と AI 参加者（MVP外）

- LiveKit：server は Apache-2.0、22,145 stars、最新 v1.13.11。Agents は Apache-2.0、15,176 stars [GH (2026/11)]。Agents はジョブ単位でルームに参加者として入る。Python と Node.js で書け、自前ホストでも動く [LiveKit Agents docs (2026/11)]。自前ホストでは信頼された CA の TLS 証明書が必要（自己署名は不可）。TURN 用のドメイン、UDP 50000〜60000 の範囲、Docker なら host network が推奨。本番では Redis を推奨する [LiveKit self-hosting (2026/11)]。ローカルの STT/LLM/TTS プラグインの対応は、このページでは確認できなかった（未確認）。Rust SDK もある（livekit/rust-sdks、508 stars）[GH (2026/11)]。
- str0m：MIT、653 stars。Sans-I/O の Rust WebRTC で、主に SFU のサーバー側で使われる。TURN とメディアのエンコード・デコードは含まない。本番の SFU には大きな設計作業が要る [str0m README (2026/11)]。
- webrtc-rs（Apache-2.0、5,184 stars）、mediasoup（ISC、7,417 stars、Node/C++）、Pion（MIT、16,894 stars、Go）[GH (2026/11)]。
- 当てはめ（筆者の判断）：AI 参加者を持つハドルなら、LiveKit Agents が最短の道。ただし Python/Node のランタイムが増え、公開 CA の証明書、UDP ポート、host network を必要とする。これは国内ローカルホスト、rootless Podman、外部への出口の既定拒否と衝突しやすい。Rust にそろえるなら str0m を自前 SFU の部品にできるが、工数は大きい。
- 判定：LiveKit は **将来の候補**（音声要件が仕様に入った時の第一候補）。str0m は **将来の候補**。mediasoup、Pion、webrtc-rs は **見送り**。

---

## 4. 通知・プッシュ

- ブラウザの Web Push：配信先の endpoint はブラウザが選ぶ [MDN Push API (2026/11)]。Safari の Web Push は Apple Push Notification service を通り、endpoint は `push.apple.com` のサブドメインになる。Apple Developer Program への加入は不要 [WebKit Meet Web Push (2022/12)]。Chrome が FCM、Firefox が Mozilla の push service を使うことは、一次資料で確認していない（未確認）。
- rust-web-push：Apache-2.0、135 stars、最終 push 2026-01-22 [GH (2026/11)]。VAPID に対応し、暗号化は ece crate を使う。Google と Mozilla の push service で試験済み。README は "still in active development" と書く [rust-web-push README (2026/11)]。
- ntfy：Apache-2.0、35,481 stars [GH (2026/11)]。UnifiedPush の配信役と Web Push に対応する。iOS では自前サーバー単独では即時通知ができない。`upstream-base-url`（通常 ntfy.sh）へ message ID と topic のハッシュだけを送り、そこから Firebase と APNs で端末を起こす [ntfy config (2026/11)]。
- UnifiedPush：利用者が配信役を選べる分散型の push。Android と Linux（KUnifiedPush）に対応する。iOS は記載がない [UnifiedPush (2026/11)]。仕様リポジトリ UnifiedPush/specifications は 110 stars、最終 push 2025-05-06 [GH (2026/11)]。
- 当てはめ（筆者の判断）：Web Push は、ブラウザが選ぶ事業者（Safari なら Apple）を経由するので、Pelway は経路の所在を選べない。v0.7 のデスクトップは OS のローカル通知で、外部 push を使わない（02）。モバイルや閉じたタブへ通知する段階では、内容を載せない「起こすだけ」の push（ntfy の poll_request と同じ型）にする。本文は Pelway server から取得させれば、12 の通知への秘密漏えいの条件（S05/S17）と両立させやすい。それでも APNs/FCM の経由は避けられず、D 系の外部送信の審査が要る。
- 判定：rust-web-push（Web）と ntfy/UnifiedPush（Android/Linux）は **将来の候補**。初期は仕様どおりローカル通知だけ。

---

## 5. ローカルの観測基盤

- OpenTelemetry Collector：Apache-2.0、7,754 stars [GH (2026/11)]。
- grafana/docker-otel-lgtm：Collector、Prometheus、Tempo、Loki、Pyroscope、Grafana を 1 コンテナにまとめたもの。"intended for development, demo, and testing environments" [docker-otel-lgtm README (2026/11)]。Grafana、Loki、Tempo は AGPL-3.0 [GH (2026/11)]。
- OpenObserve：AGPL-3.0、23,010 stars。Rust 製 [GH (2026/11)] [OpenObserve docs (2026/11)]。単一バイナリかどうかと、エディションの差は未確認。
- SigNoz：本体は MIT、`ee/` は別ライセンス [SigNoz LICENSE (2026/11)]。32,590 stars [GH (2026/11)]。保存先の構成（ClickHouse の要否）は未確認。
- Vector：MPL-2.0、22,855 stars [GH (2026/11)]。
- 当てはめ（筆者の判断）：v0.7 は tracing のローカル出力で、外部 exporter を初期無効とする（02/09）。運営者一人の 1 ホストなら、JSON ログをファイルに出してローテーションする形で足りる。OTLP の受け口は、Collector を loopback の file exporter だけで動かせば、外部出口なしで将来の相互運用に備えられる。LGTM は開発用と明記されており、AGPL を同梱しないためにも本番に置かない。
- 判定：OTel Collector は **将来の候補**。docker-otel-lgtm は開発機での試用に限り **将来の候補**。OpenObserve、SigNoz、Vector は **見送り**（1 ホストには過剰）。

---

## 6. クライアントの部品

### 6.1 Slint（A1）

- 事実：Slint は 24,559 stars、最新 1.18.1（2026-09-21）[GH (2026/11)]。ListView は "elements are only instantiated if they are visible" [Slint ListView (2026/11)]。可変高の項目への対応は、このページに記述がない（未確認）。1.16 で StyledText と `@markdown(...)` が入り、"a first step toward full rich text support" とされた [Slint 1.16 (2026/04)]。StyledText のテキスト選択の可否は未確認。
- 既知の issue（open）：#3867 は「`accessibility` 機能が大きな ListView で性能に大きく影響する」。報告は Slint 1.3.0 に対するもので、2026-10-11 に更新されている [slint #3867 (2026/11)]。1.18.1 で再現するかは未確認。#10861 は「LineEdit で IME の変換中に窓のフォーカスが外れると最後の文字が落ちる」[slint #10861 (2026/11)]。#3949 は「TextInput の選択範囲を取得・設定する API」[slint #3949 (2026/11)]。
- 「Slint でのチャット」の公開実装は、GitHub 検索では最大 36 stars の 2023 年のデモしか見つからなかった [GH search (2026/11)]。
- アクセシビリティの公式ページは取得できなかった（404）。AccessKit は BSD-3-Clause、1,558 stars [GH (2026/11)]。
- 判定：**MVP前に検証**（A1）。#3867 と #10861 は A1 の検証項目（大量可変高リストの読み上げ、IME）にそのまま当たるので、合成データで再現試験を先に行う。

### 6.2 Tauri 2（比較候補）

- 事実：安定版は tauri v2.12.5（2026-10-21）。v3 は alpha の pre-release [GH releases (2026/11)]。112,753 stars、Apache-2.0 [GH (2026/11)]。wry の PR #1724（open）は、wry が Linux で "disables IME preedit unconditionally" だと指摘し、設定可能にする変更を提案している。報告者は Korean 入力と fcitx で問題を確認した [wry #1724 (2026/11)]。
- 当てはめ（筆者の判断）：Tauri の読み上げと IME の品質は各 OS の WebView（WebView2 / WKWebView / WebKitGTK）に依存する。Linux の日本語のインライン変換は、#1724 が入るまで要注意。Web 版と UI を共有できる利点は、A2 の「UI部品は共有しない」方針と比べて評価する必要がある。
- 判定：比較候補として **MVP前に検証**（Linux の fcitx5 + mozc での変換中表示、Orca）。

### 6.3 Rust↔TS の型生成と WASM サイズ（A2）

- ts-rs：MIT、1,948 stars。specta：MIT、654 stars。typeshare：Apache-2.0、2,992 stars。tauri-specta：817 stars [GH (2026/11)]。
- wasm-bindgen：9,227 stars（リポジトリは wasm-bindgen/wasm-bindgen へ移った）。binaryen（wasm-opt）：8,680 stars、活動中。twiggy（サイズ分析）は archived [GH (2026/11)]。
- 当てはめ（筆者の判断）：A2 の「型生成維持費」の検証には、API 型を ts-rs か specta の一方で出力し、CI で差分を検査する試作が最小になる。typeshare は複数言語向けで、Pelway には過剰かもしれない（未検証）。twiggy が archived なので、サイズの内訳は wasm-opt の出力サイズと `cargo bloat` などで代える必要がある（代替手段の適否は未確認）。
- 判定：ts-rs / specta は **MVP前に検証**（A2）。wasm-opt も **MVP前に検証**（A2 のサイズ測定）。

### 6.4 日本語 i18n・Fluent

- 事実：pelway-client-ui は `fluent-bundle = "0.15"` と `unic-langid` を使う。コメントには「@tr()（gettext）は使わない」とある（`crates/pelway-client-ui/Cargo.toml`）。fluent-rs は Apache-2.0、1,378 stars、最終 push 2026-09-27。fluent.js は 1,011 stars、最終 push 2026-05-22。cargo-i18n は 148 stars [GH (2026/11)]。
- 当てはめ（筆者の判断）：Web（React）側で同じ `.ftl` を読むには、fluent.js 系（@fluent/bundle）を使えば資源を共有できる。翻訳キーの欠落検査は、Rust 側で ja と en のメッセージ ID の差分をテストで検査するのが軽い。
- 判定：fluent.js による `.ftl` の共有は **MVP前に検証**（A2 の「翻訳資源を共有」）。

### 6.5 Octane（Web クライアントの UI 候補）

- 事実：octanejs/octane。MIT、1,485 stars、最終 push 2026-11-12。npm 版 0.16.7（2026-11-02）、状態は beta [GH (2026/11)] [npm (2026/11)]。React の API（`useState`、`useEffect`、`memo`、context、portals、Suspense、transition）を互換で提供し、コンパイラが直接 DOM のコードへ落とす（仮想 DOM を持たない）。Inferno の後継で、作者は Inferno の作者。React との差は、クラスコンポーネント・Server Components・合成イベントシステムを持たないこと [octane README (2026/11)]。
- 当てはめ（筆者の判断）：A2 の UI 層（TS/React DOM）の候補。React と同じ API で書けるので学習の負担は React と同じだが、仮想 DOM を持たない分、描画の負荷は下回る可能性がある（実 DOM での測定は未実施）。型生成と WASM のサイズは UI フレームワークに依らないので、採否にかかわらず結果は変わらない [slint-tauri-client-measurements (2026/11)]。
- 判定：Octane は **将来の候補**（A2 の UI 層の比較対象）。実 DOM での読み上げ・IME・描画負荷の比較は別 Issue に残す。

---

## 7. 相互接続・ブリッジ

- Matrix：Synapse は AGPL-3.0、4,786 stars。tuwunel（conduwuit の後継、Rust）は Apache-2.0、2,762 stars。continuwuity は Apache-2.0、1,171 stars。ruma（Rust の Matrix 型群）は MIT、748 stars。matrix-rust-sdk は Apache-2.0 [GH (2026/11)]。
- ブリッジ：mautrix/slack は "A Matrix-Slack puppeting bridge"、AGPL-3.0、最終 push 2026-11-11 [GH (2026/11)]。Matrix の公式ブリッジ一覧は Slack 向けを 1 つ載せ、Teams 向けは載せていない [Matrix bridges (2026/11)]。保守されている Teams ブリッジは見つからなかった（GitHub 検索では 2020 年で止まった 0 stars のものだけ）[GH search (2026/11)]。
- 参考実装：Zulip は Apache-2.0、26,207 stars [GH (2026/11)]。Mattermost のソースは AGPL-3.0 か商用ライセンスで、Mattermost 社がビルドしたバイナリは MIT、一部ディレクトリは Apache-2.0 [Mattermost LICENSE (2026/11)]。
- 当てはめ（筆者の判断）：Matrix のフェデレーションは他の組織のサーバーへメッセージを複製するため、Pelway の国内保存、取消、削除、可視範囲の継承（12 §4/§5）と根本的に衝突する。Slack ブリッジは AGPL のため、別プロセスとして分ける必要があり、Teams ブリッジは保守されたものがない。外部チャットとつなぐなら、A9 の IntegrationPort に Slack/Teams の Adapter を置く方が、承認と監査の境界を保てる。
- 判定：Matrix 互換フェデレーションは **見送り**（09 の参考扱いを維持）。ruma の型は同期設計の参考として **将来の候補**。Zulip と Mattermost は設計の参考に限る。

---

## 8. 判定一覧

| 候補 | 判定 | 関係する項目 |
|---|---|---|
| webauthn-rs | MVP前に検証 | U05 |
| openidconnect-rs | MVP前に検証 | U05 |
| Slint の #3867 / #10861 の再現 | MVP前に検証 | A1 |
| Tauri 2（Linux IME、Orca） | MVP前に検証（比較候補） | A1 比較 |
| ts-rs / specta、wasm-opt | MVP前に検証 | A2 |
| fluent.js による .ftl の共有 | MVP前に検証 | A2 / 06 |
| Octane（Web クライアントの UI 候補） | 将来の候補（比較対象） | A2 |
| Ory Kratos (+Hydra) | 将来の候補 | U05 |
| Kanidm / Rauthy / Keycloak | 将来の候補 | U05 |
| Ory Keto / OpenFGA / SpiceDB | 見送り | 12 §7 |
| Zitadel / Authentik / Oathkeeper / Polis | 見送り | U05 |
| OpenMLS | 将来の候補 | 12 §7 |
| vodozemac | 見送り | — |
| LiveKit (+Agents) / str0m | 将来の候補 | 要件なし |
| mediasoup / Pion / webrtc-rs | 見送り | — |
| rust-web-push / ntfy / UnifiedPush | 将来の候補 | 02 |
| OTel Collector（loopback のみ） | 将来の候補 | 02 / 09 |
| OpenObserve / SigNoz / Vector | 見送り | — |
| Matrix フェデレーション / ブリッジ | 見送り | 09 |

---

## 出典

[Pelway v0.7 02, 2026/11] Pelway. "02 技術比較と依存採用案." リポジトリ内. docs/technical/v0.7/body/02-stack-comparison.md
[Pelway v0.7 08, 2026/11] Pelway. "08 採用記録・未決の正本・受入試験." リポジトリ内. docs/technical/v0.7/body/08-decisions-open.md
[Pelway v0.7 09, 2026/11] Pelway. "09 出典." リポジトリ内. docs/technical/v0.7/body/09-sources.md
[Pelway v0.7 12, 2026/11] Pelway. "12 通信秘密・目的別根拠・可視範囲・運営アクセス." リポジトリ内. docs/technical/v0.7/body/12-communication-secrecy.md
[GH, 2026/11] GitHub. 各リポジトリのメタデータ（gh repo view / gh api repos/{owner}/{repo}/license / gh release list）. https://github.com
[GH search, 2026/11] GitHub. "gh search repos 'slint chat' / 'matrix teams bridge'." https://github.com/search
[webauthn-rs README, 2026/11] kanidm. "webauthn-rs README." GitHub. https://github.com/kanidm/webauthn-rs
[openidconnect-rs README, 2026/11] ramosbugs. "openidconnect-rs README." GitHub. https://github.com/ramosbugs/openidconnect-rs
[Kanidm OAuth2, 2026/11] Kanidm. "OAuth2." Kanidm Book. https://kanidm.github.io/kanidm/stable/integrations/oauth2.html
[Kanidm book, 2026/11] Kanidm. "Kanidm Book (print)." https://kanidm.github.io/kanidm/master/print.html
[Kanidm book src, 2026/11] kanidm. "book/src." GitHub. https://github.com/kanidm/kanidm/tree/master/book/src
[Rauthy README, 2026/11] sebadob. "rauthy." GitHub. https://github.com/sebadob/rauthy
[Rauthy book, 2026/11] sebadob. "Upstream Authentication Providers." GitHub. https://github.com/sebadob/rauthy/blob/main/book/src/auth_providers/index.md
[Ory OSS v25.4.0, 2025/11] Ory. "Announcing Ory OSS v25.4.0." Ory Blog. https://ory.com/blog/ory-oss-v25-4-0-new-release-announcement-versioning-scheme
[Ory OEL docs, 2026/11] Ory. "Ory Enterprise License." Ory Docs. https://www.ory.com/docs/self-hosted/oel
[Kratos README, 2026/11] Ory. "ory/kratos README." GitHub. https://github.com/ory/kratos
[Hydra README, 2026/11] Ory. "ory/hydra README." GitHub. https://github.com/ory/hydra
[Keto README, 2026/11] Ory. "ory/keto README." GitHub. https://github.com/ory/keto
[Ory case study OpenAI, 2026/11] Ory. "OpenAI case study." https://www.ory.com/case-studies/openai
[Ory blog OpenAI, 2026/11] Ory. "OpenAI OAuth2 server open source." Ory Blog. https://www.ory.com/blog/openai-oauth2-server-open-source
[Kratos passkeys, 2026/11] Ory. "Passkeys." Ory Docs. https://www.ory.com/docs/kratos/passwordless/passkeys
[Kratos social sign-in, 2026/11] Ory. "Social sign-in overview." Ory Docs. https://www.ory.com/docs/kratos/social-signin/overview
[Kratos deploy example, 2026/11] Ory. "Deploy Kratos example." Ory Docs. https://www.ory.com/docs/kratos/guides/deploy-kratos-example
[ory/elements locales, 2026/11] Ory. "elements-react locales." GitHub. https://github.com/ory/elements/tree/main/packages/elements-react/src/locales
[Ory blog OAuth for AI agents, 2026/11] Ory. "OAuth for AI Agents: How Authorization Actually Works." Ory Blog. https://www.ory.com/blog/oauth-for-ai-agents
[ory/hydra #1218, 2026/11] Ory. "Support RFC8693: Token Exchange." GitHub Issue. https://github.com/ory/hydra/issues/1218
[ory/hydra #4134, 2026/11] Ory. "RFC: Implement OAuth Client ID Metadata Documents (CIMD) for MCP clients." GitHub Issue. https://github.com/ory/hydra/issues/4134
[Keto docs, 2026/11] Ory. "Ory Keto." Ory Docs. https://www.ory.com/docs/keto/
[OpenFGA config, 2026/11] OpenFGA. "Configure OpenFGA." https://openfga.dev/docs/getting-started/setup-openfga/configure-openfga
[SpiceDB datastores, 2026/11] AuthZed. "Datastores." https://authzed.com/docs/spicedb/concepts/datastores
[Zitadel AGPL, 2025/03] ZITADEL. "Apache to AGPL." ZITADEL Blog. https://zitadel.com/blog/apache-to-agpl
[Keycloak DB, 2026/11] Keycloak. "Configuring the database." https://www.keycloak.org/server/db
[Authentik LICENSE, 2026/11] goauthentik. "LICENSE." GitHub. https://github.com/goauthentik/authentik/blob/main/LICENSE
[RFC 9420, 2023/07] Barnes, R. et al. "The Messaging Layer Security (MLS) Protocol." IETF. https://www.rfc-editor.org/rfc/rfc9420.html
[vodozemac README, 2026/11] matrix-org. "vodozemac." GitHub. https://github.com/matrix-org/vodozemac
[LiveKit Agents docs, 2026/11] LiveKit. "Agents." https://docs.livekit.io/agents/
[LiveKit self-hosting, 2026/11] LiveKit. "Deployment." https://docs.livekit.io/home/self-hosting/deployment/
[str0m README, 2026/11] algesten. "str0m." GitHub. https://github.com/algesten/str0m
[MDN Push API, 2026/11] MDN. "Push API." https://developer.mozilla.org/en-US/docs/Web/API/Push_API
[WebKit Meet Web Push, 2022/12] WebKit. "Meet Web Push." WebKit Blog. https://webkit.org/blog/12945/meet-web-push/
[rust-web-push README, 2026/11] pimeys. "rust-web-push." GitHub. https://github.com/pimeys/rust-web-push
[ntfy config, 2026/11] ntfy. "Configuration." https://docs.ntfy.sh/config/
[UnifiedPush, 2026/11] UnifiedPush. "UnifiedPush." https://unifiedpush.org/
[docker-otel-lgtm README, 2026/11] Grafana Labs. "docker-otel-lgtm." GitHub. https://github.com/grafana/docker-otel-lgtm
[OpenObserve docs, 2026/11] OpenObserve. "Documentation." https://openobserve.ai/docs/
[SigNoz LICENSE, 2026/11] SigNoz. "LICENSE." GitHub. https://github.com/SigNoz/signoz/blob/main/LICENSE
[Slint ListView, 2026/11] Slint. "ListView." https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/listview/
[Slint 1.16, 2026/04] Slint. "Slint 1.16 Released." https://slint.dev/blog/slint-1.16-released
[slint #3867, 2026/11] slint-ui. "accessibility feature have big performance impact on big listview." GitHub Issue. https://github.com/slint-ui/slint/issues/3867
[slint #10861, 2026/11] slint-ui. "The last character is dropped when the window loses focus during IME composition in LineEdit." GitHub Issue. https://github.com/slint-ui/slint/issues/10861
[slint #3949, 2026/11] slint-ui. "TextInput: add API to get and set the selection." GitHub Issue. https://github.com/slint-ui/slint/issues/3949
[GH releases, 2026/11] tauri-apps. "tauri releases." GitHub. https://github.com/tauri-apps/tauri/releases
[wry #1724, 2026/11] tauri-apps. "feat(webkitgtk): make input method preedit configurable." GitHub PR. https://github.com/tauri-apps/wry/pull/1724
[Matrix bridges, 2026/11] Matrix.org. "Bridges." https://matrix.org/ecosystem/bridges/
[Mattermost LICENSE, 2026/11] Mattermost. "LICENSE.txt." GitHub. https://github.com/mattermost/mattermost/blob/master/LICENSE.txt
[octane README, 2026/11] octanejs. "octane README." GitHub. https://github.com/octanejs/octane
[npm, 2026/11] npm. "octane@0.16.7" のパッケージ情報. https://www.npmjs.com/package/octane

## 付録：リポジトリ一覧（GH (2026/11) の根拠）

確認日 2026-11-11。stars と最終 push の日付は変動する。

| リポジトリ | ライセンス | stars | 最終 push |
|---|---|---|---|
| https://github.com/kanidm/kanidm | MPL-2.0 | 5,559 | 2026-11-10 |
| https://github.com/zitadel/zitadel | AGPL-3.0 | 15,559 | 2026-11-11 |
| https://github.com/keycloak/keycloak | Apache-2.0 | 37,821 | 2026-11-11 |
| https://github.com/goauthentik/authentik | MIT＋enterprise | 26,307 | 2026-11-11 |
| https://github.com/ory/kratos | Apache-2.0 | 13,954 | 2026-07-29 |
| https://github.com/ory/hydra | Apache-2.0 | 17,659 | 2026-07-29 |
| https://github.com/ory/keto | Apache-2.0 | 5,422 | 2026-09-04 |
| https://github.com/ory/oathkeeper | Apache-2.0 | 3,611 | 2026-07-27 |
| https://github.com/ory/polis | Apache-2.0 | 2,272 | 2026-07-27 |
| https://github.com/ory/elements | Apache-2.0 | 188 | 2026-11-10 |
| https://github.com/openfga/openfga | Apache-2.0 | 6,131 | 2026-11-11 |
| https://github.com/authzed/spicedb | Apache-2.0 | 7,215 | 2026-11-11 |
| https://github.com/sebadob/rauthy | Apache-2.0 | 1,443 | 2026-11-11 |
| https://github.com/kanidm/webauthn-rs | MPL-2.0 | 721 | 2026-11-05 |
| https://github.com/ramosbugs/openidconnect-rs | MIT | 652 | 2026-10-31 |
| https://github.com/openmls/openmls | MIT | 1,065 | 2026-11-11 |
| https://github.com/matrix-org/vodozemac | Apache-2.0 | 416 | 2026-11-11 |
| https://github.com/livekit/livekit | Apache-2.0 | 22,145 | 2026-11-11 |
| https://github.com/livekit/agents | Apache-2.0 | 15,176 | 2026-11-11 |
| https://github.com/livekit/rust-sdks | 未確認 | 508 | 2026-11-11 |
| https://github.com/versatica/mediasoup | ISC | 7,417 | 2026-11-11 |
| https://github.com/pion/webrtc | MIT | 16,894 | 2026-11-10 |
| https://github.com/algesten/str0m | MIT | 653 | 2026-11-10 |
| https://github.com/webrtc-rs/webrtc | Apache-2.0 | 5,184 | 2026-11-07 |
| https://github.com/binwiederhier/ntfy | Apache-2.0 | 35,481 | 2026-11-11 |
| https://github.com/UnifiedPush/specifications | 未確認 | 110 | 2025-05-06 |
| https://github.com/pimeys/rust-web-push | Apache-2.0 | 135 | 2026-01-22 |
| https://github.com/open-telemetry/opentelemetry-collector | Apache-2.0 | 7,754 | 2026-11-10 |
| https://github.com/grafana/docker-otel-lgtm | Apache-2.0 | 未取得 | 未取得 |
| https://github.com/grafana/grafana | AGPL-3.0 | 77,831 | 2026-11-11 |
| https://github.com/grafana/loki | AGPL-3.0 | 29,155 | 2026-11-11 |
| https://github.com/grafana/tempo | AGPL-3.0 | 5,570 | 2026-11-11 |
| https://github.com/openobserve/openobserve | AGPL-3.0 | 23,010 | 2026-11-11 |
| https://github.com/SigNoz/signoz | MIT＋ee | 32,590 | 2026-11-11 |
| https://github.com/vectordotdev/vector | MPL-2.0 | 22,855 | 2026-11-11 |
| https://github.com/slint-ui/slint | 複数（GPL/商用/Royalty-free） | 24,559 | 2026-11-11 |
| https://github.com/tauri-apps/tauri | Apache-2.0 | 112,753 | 2026-11-10 |
| https://github.com/AccessKit/accesskit | BSD-3-Clause | 1,558 | 2026-11-09 |
| https://github.com/Aleph-Alpha/ts-rs | MIT | 1,948 | 2026-11-09 |
| https://github.com/specta-rs/specta | MIT | 654 | 2026-11-10 |
| https://github.com/specta-rs/tauri-specta | 未確認 | 817 | 2026-11-11 |
| https://github.com/1Password/typeshare | Apache-2.0 | 2,992 | 2026-11-08 |
| https://github.com/wasm-bindgen/wasm-bindgen | 未確認 | 9,227 | 2026-11-11 |
| https://github.com/WebAssembly/binaryen | 未確認 | 8,680 | 2026-11-11 |
| https://github.com/rustwasm/twiggy | 未確認（archived） | 1,424 | 2026-02-18 |
| https://github.com/projectfluent/fluent-rs | Apache-2.0 | 1,378 | 2026-09-27 |
| https://github.com/projectfluent/fluent.js | 未確認 | 1,011 | 2026-05-22 |
| https://github.com/element-hq/synapse | AGPL-3.0 | 4,786 | 2026-11-11 |
| https://github.com/matrix-construct/tuwunel | Apache-2.0 | 2,762 | 2026-11-11 |
| https://github.com/continuwuity/continuwuity | Apache-2.0 | 1,171 | 2026-11-11 |
| https://github.com/ruma/ruma | MIT | 748 | 2026-11-11 |
| https://github.com/matrix-org/matrix-rust-sdk | Apache-2.0 | 未取得 | 未取得 |
| https://github.com/mautrix/slack | AGPL-3.0 | 107 | 2026-11-11 |
| https://github.com/zulip/zulip | Apache-2.0 | 26,207 | 2026-11-11 |
| https://github.com/mattermost/mattermost | AGPL-3.0/商用＋MIT（バイナリ） | 39,579 | 2026-11-11 |

Slint のライセンス条文の詳細は [Pelway v0.7 02 (2026/11), Slintのライセンス] を参照。
