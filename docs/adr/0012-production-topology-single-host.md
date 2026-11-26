---
status: accepted
---

# 本番トポロジはCompose＋リバースプロキシ＋systemdタイマーの単一ホスト構成を雛形にする

Pelway の本番配置の雛形は、クラウド固有サービスに依存しない単一ホスト構成とする。構成要素は Docker Compose で宣言し、公開入口は単一のリバースプロキシ（Caddy）、ホスト運用の定期処理は systemd タイマーで回す。Rakazo の `infra/compose`・`infra/systemd`（[elie222/rakazo](https://github.com/elie222/rakazo)）を雛形に、Pelway の構成要素（DB キュー等）へ翻案する。この ADR は本番配置の設計方針であり、実際のホスティング契約・配置作業・compose ファイルの作成は対象外（#202 の対象外と同じ）。

## 比較した方式

- **Rakazo のセルフホスト構成（採用の雛形）**: 本番用 Compose（api・worker・postgres・migrate・リバースプロキシ）、Caddy のリバースプロキシ、ホスト堅牢化スクリプト（harden-host・firewall・apt-security）、systemd の定期チェックまでをリポジトリで管理する。単一ホストに全部が載り、国内 VPS へそのまま置ける。この「インフラをリポジトリのコードとして持つ」形を採る。
- **OpenBot の Kubernetes 既定（不採用）**: 初期規模には過剰であり、単一ホストで足りる間はオーケストレータの運用負担（クラスタ自身の管理・監視・更新）を負わない。規模が必要になった時点で移行設計を別に立てる。
- **CircleChat の最小 Compose（下限の実例、構成は採用しない）**: Postgres＋Redis＋MinIO の3サービス。Pelway は Redis を使わない DB キュー（[ADR 0006](0006-db-queue-choice.md)）を選び、添付・成果物はクラウドのオブジェクトストレージではなくホストのローカルボリュームに置くため、どちらも不要。

## 構成要素と配置

単一ホスト上の Compose プロジェクトとして次を配置する。プロセス分離は [技術仕様 v0.7 の01](../technical/v0.7/body/01-architecture.md)（server・runner・gateway は論理境界として別プロセス）を本番でも維持する。

| 構成要素 | 配置 | 役割 |
|---|---|---|
| リバースプロキシ（Caddy） | ホスト唯一の公開入口。`edge` ネットワーク | TLS の終端、API への転送、ブラウザ UI 静的資産の配信、アクセスログのマスキング（認証ヘッダ・トークン類を落とす） |
| API（server: axum） | コンテナ。`edge`/`app`/`data` | 認証と承認、状態・監査・outbox の操作 |
| ワーカー（runner） | コンテナ。`app`/`data` | DB キュー（ADR 0006）のジョブ実行、lease と手順の再開 |
| Postgres | コンテナ。`data` のみ | 状態・監査・outbox・DB キューの正本 |
| サンドボックス管理（sandbox controller と task sandbox） | controller は `app` と `sandbox`、task sandbox は `sandbox` のみ | タスクごとの Linux コンテナ sandbox（秘密なし・外部出口なし）の起動と管理。runner から controller への起動依頼は `app` 経由（[v0.7 01](../technical/v0.7/body/01-architecture.md) の `RUN --> CTRL`） |
| 定期ジョブ（systemd タイマー） | ホスト側 | 国内 backup（13章 R15: 毎日 02:00 JST、snapshot は作成から14日）、ホストヘルスチェック、セキュリティ更新 |

補足:

- ネットワークは `edge`（リバースプロキシ↔API）、`app`（API・ワーカー・ゲートウェイ間に加え、runner↔sandbox controller の制御面）、`data`（API・ワーカー↔Postgres）、`sandbox`（controller・連携 gateway↔task sandbox）に分ける。[05 ローカル環境](../technical/v0.7/body/05-local-dev.md)の「`app` と `sandbox` を分け、DB は sandbox から到達不能にする」を本番でも維持する。このため sandbox controller は制御面の `app` と `sandbox` の両方に置き、task sandbox は `sandbox` のみに置いて `data` には参加させない（sandbox から Postgres への到達経路を作らない）。Postgres と sandbox に `ports` を持たせない（ホストに公開しない）。
- マイグレーションはワンショットの migrate サービス（owner 接続はここだけ、適用したら終わる）で当てる。LLM ゲートウェイは `app` 側の別プロセス。連携 gateway は `app` と `sandbox` の両方に置く: task sandbox からの操作要求（v0.7 01 の `SB --> GW`）は `sandbox` 経由で受け、承認と attempt 記録のための API との往復は `app` 経由で行う。task sandbox から見た `sandbox` 上の隣は controller と連携 gateway だけで、sandbox の外部出口は連携 gateway だけ（v0.7 01 の「外部出口はGWだけ」）のままにする。添付・成果物・バックアップはホストのローカルボリュームに置く。
- コンテナには公開入口のリバースプロキシも含めて `restart: unless-stopped`・`no-new-privileges:true`・`cap_drop: ALL`（必要なものだけ戻す）・リソース制限（pids・memory）・ログ上限を付け、Rakazo の本番 Compose と同じ守りを雛形にする。
- プロダクト機能の定期実行（ルーティン）は DB キュー経由で server・runner が処理する（ADR 0010・0011）。この systemd タイマーはホスト運用の定期ジョブ専用であり、両者を混ぜない。

## 国内ホスティングとクラウド非依存

- 本番は国内のホスティング（国内 VPS または国内リージョン）に置く前提とする。ADR 0008 の国内完結の方針を本番配置にも適用する。ホスティング先の選定は未決（この ADR では VPS か国内リージョンかを決めない）。
- マネージド DB・マネージドメッセージング・オブジェクトストレージ等のクラウド固有サービスには依存しない。構成要素はすべてホスト上のコンテナとローカルボリュームで成立させ、国内のホスティングへそのまま置ける移植性を保つ。
- 外向き通信は、ADR 0008 の BYOK（契約者の責任による例外）でテナントが有効化した LLM 提供元への通信（LLM ゲートウェイ経由）と、利用者が許可した連携 gateway の transport（v0.7 01 の「許可済みtransportだけ」。国外処理があり得る例外経路）の 2 つだけに限る。それ以外の構成要素に外向き通信を与えない。（[ADR 0014](0014-desktop-client-electron.md) で、デスクトップアプリの配布のために、ホストの取得の処理が GitHub Releases・タグ・attestation を取りに行く経路を足した。server・runner・gateway などの構成要素には与えない。）

## ホスト堅牢化はコードとして管理する

ホストの堅牢化を手作業で行わず、リポジトリ内のスクリプトと設定として管理する（Rakazo の `infra/compose` と同じ置き方）。対象は次のとおり。

- ファイアウォール（nftables/ufw）: 公開するのは SSH・HTTP・HTTPS だけ。それ以外のポートを遮断する。コンテナサービスはホストにポートを出さない。
- 自動セキュリティ更新（unattended-upgrades）: セキュリティ更新を自動で当てる。
- SSH の硬直化: root ログイン禁止、鍵認証のみ、ログイン可能ユーザの限定。
- 侵入検知と監視の補助（fail2ban・apparmor・auditd）。
- systemd タイマーのユニットファイル（backup・host-check）もリポジトリで管理し、ホスト側の定期処理を手作業にしない。

## Consequences

- 配置作業とホスティング契約は別の人手 Issue とする。Kubernetes 等への移行設計は、規模が必要になってから立てる。
- 本番用 compose ファイル・堅牢化スクリプト・systemd ユニットの作成は、この ADR を前提にした別 Issue で行う。この ADR の時点ではトポロジの構成要素と配置・守りの方針だけを決める。
- [技術仕様 v0.7](../technical/v0.7/README.md) は国内ローカルホストの現行正本のまま変わらず、この ADR は本番配置の雛形を定める。両方は矛盾しない（v0.7 01 の「クラウド本番を構築しない」は現時点で構築しないことを指す）。
