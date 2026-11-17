# 連携カタログ（Composio／Pipedream）の評価と採用可否の判断材料

確認日：2026-11-16（JST、日本時間）。価格・プランはこの時点の公式ページの表示。

## 推奨案（1つ）

**MVPでは連携カタログ（Composio／Pipedream）を採用せず、自前実装とする（連携はロードマップどおり検証対象のみ）。カタログ利用は、ADR-0008 の BYOK と同じ例外枠（テナント単位・初期オフ・管理者の明示的有効化・国外配送と資格情報預託の表示・監査）で将来、個別に検討する。**

要約すると、(1) 両カタログの実行経路は米国（Pipedream は us-east-1 を公式明記、Composio は地域を明記しないが公表の送信元 IP が全て AWS us-east-1）で、国内完結（ADR-0008）の既定と両立しない、(2) どちらも操作の段階（ADR-0004 の4段階）に相当するメタデータをツールに持たず、カタログの連携幅を ADR-0004 の宣言運用で活かせない、(3) 費用自体は障壁にならない、の3点から、MVP の連携数（2〜3個）では自前実装が方針適合・工数の両面で優る。採用の最終判断は hatai が行う（Issue #188 の未決事項）。

## この文書の前提と読み方

- 出典は公式資料に限定した：Composio の公式サイト・公式ドキュメント・公式リポジトリ、Pipedream の公式サイト・公式ドキュメント、Amazon の公式 IP 範囲ファイル。第三者比較記事は使っていない。
- 「確認」は公式資料に記載があるという意味で、実機検証済み・契約審査済みではない。「未確認」は今回の資料で裏付けられなかった項目で、機能がないという意味ではない（[競合調査](../competitors.md)と同じ流儀）。
- 比較の対象は、Pelway が外部連携の操作と資格情報管理をカタログ事業者に委託する使い方（Rakazo が Composio／Pipedream を使う形。セルフホストでカタログを Composio／Pipedream から設定する点は[競合調査の Rakazo](../competitors/rakazo.md) の確認済み記述で、公式文書も「セルフホストでは運営者がカタログを設定した場合のみ表示」[integrations][rakazo-integrations] と説明する）。Composio の個人向けプラン（For You）と Pipedream のワークフロー製品は補足に留める。
- 「自前実装」の列は Pelway の設計（[技術文書 v0.7 の10章](../technical/v0.7/body/10-domestic-only.md)、[ADR 0004](../adr/0004-declared-action-tiers.md)、[ADR 0008](../adr/0008-llm-provider-and-byok.md)）に基づく構想であり、実装済みではない。
- Composio の価格は2026-08-15 以降の新規契約に適用される新価格（公式価格ページの注記）。

## 比較表

| 軸 | Composio | Pipedream（Connect） | 自前実装（Pelway 構想） |
|---|---|---|---|
| 資格情報の保管方式 | Composio Cloud が保管し、実行時にサーバー側で注入。AES-256-GCM で保存時暗号化・転送中 TLS、API 応答では既定でマスク。アプリにもモデルにもトークンを出さない。独自 OAuth アプリにしても保管は Composio 側。実行時に都度渡す「Self-managed credentials」（保存しない）と、顧客管理鍵（KMS、Composio は平文を扱えない proxy 構成）は Platform Enterprise 向け [token custody][composio-token-custody] [価格][composio-pricing-platform] | Pipedream が保管。OAuth 付与・API キー・環境変数を AWS KMS の鍵で AES-256-GCM 保存時暗号化（鍵は Pipedream 管理、年1回ローテーション）。実行は Firecracker microVM、MCP と Connect proxy も Pipedream のサーバー経由で、モデルに資格情報を出さない [security][pipedream-security] [MCP][pipedream-mcp] | 技術文書10章の gateway＋`integration_grant` の枠で国内 DB に暗号化保存（参考: Rakazo は Postgres 内で AES-256-GCM 暗号化＋origin 完全一致で復号注入）。sandbox は秘密なし・gateway 経由でモデルに渡さない [10章][pelway-ch10] [Rakazo資格情報][rakazo-secrets] |
| データの経由地域 | 処理・保存地域の公式記述は未確認。送信元 IP 4件を公式公表しており、Amazon 公式 ip-ranges.json で全て us-east-1 に割り当て。既定で要求引数・応答本体を実行ログに最長1年保持（ZDR は有料アドオンで保存停止） [outbound IPs][composio-outbound-ips] [data retention][composio-data-retention] | 公式に「AWS us-east-1 でホスト」と明記。egress も us-east-1 の AWS 標準レンジ。Connect（API/MCP）では要求・応答本体を保存・記録しない [security][pipedream-security] | 国内 sandbox・gateway 経由の設計。10章の台帳で「国内保存」と「国外送信なし」を分けて管理 [10章][pelway-ch10] |
| 段階宣言（ADR-0004）との適合 | ツール定義は名前・説明・入力スキーマで、操作の段階に相当するメタデータの提供は今回の資料では確認できず。ツールごとの必須 OAuth スコープを問い合わせる API はある。Firewall は自然文ポリシーによる呼び出し検査・ブロック（有料アドオン）で、段階の確定は Pelway 側に残る [SDK docs][composio-docs] [価格][composio-pricing-platform] | 登録済みアクションから MCP ツールを自動生成。段階に相当するメタデータの提供は今回の資料では確認できず。Conduit のアクセスポリシーはコネクタ・ツール単位の許可リストで段階の概念なし [MCP][pipedream-mcp] [Conduit connectors][pipedream-conduit-connectors] | ADR-0004 の宣言レジストリと管理者確定を最初から実装に含められる。段階が決まっていない操作は止める（fail closed）[ADR 0004][pelway-adr4] |
| 費用 | Platform: Hobby $0（tool calls 10万回/月・トリガー5万回/月・メンバー3・上限で停止）、Pro $29/月（超過 tool call $0.0003/回・トリガー $0.003/回、管理アプリは別枠で $0.0005〜$0.01/回、接続1,000件まで無料その後 $0.10/件、ZDR・Firewall は従量アドオン）、Enterprise 個別 [価格][composio-pricing-platform] | Free $0（開発環境のみ。3,000+ コネクタ・10,000+ ツール）、Startup $99/月（年払い。月払いは$150。本番環境、外部ユーザー100まで・超過 $2/ユーザー、クレジット1万まで・超過 $0.012）、Business 個別（年契約。SSO・SCIM・HIPAA・専有静的 egress IP） [価格][pipedream-pricing] | 外部課金なし。[技術文書§7](../technical.md) の見積もりで連携2〜3個を含む段階3が3〜4週（2〜3人）。連携ごとの API 変更追従と OAuth アプリ管理の維持費用が残る |

補足:

- Composio の SOC 2 Type II・ISO 27001:2022、Pipedream の SOC 2 Type 2・GDPR（SCC 付き DPA）・HIPAA BAA は各公式文書で確認 [enterprise][composio-enterprise] [security][pipedream-security]。
- どちらのカタログも「payload をモデルの学習に使わない」（Composio は全プランで明記、Pipedream は Connect の保存なしで示唆）と公式に述べる [data retention][composio-data-retention] [security][pipedream-security]。
- Pipedream のワークフロー製品（String を含む）は 2027-03-31 に終了予定で、Connect は継続 [workflows EOL][pipedream-workflows-eol]。Composio・Pipedream とも日本リージョンの提供は今回の資料では確認できず。
- 公式リポジトリ: `ComposioHQ/composio`（MIT、SDK モノレポでバックエンドのソースは含まない）、`PipedreamHQ/pipedream`（コンポーネント registry。ライセンスは OSS 承認外、プラットフォーム本体は非公開） [repo][composio-repo] [repo][pipedream-repo]

## ADR-0008（国内完結）と両立する使い方の整理

判断の枠は技術文書10章が既に持つ。「利用者が選ぶ外部サービス連携は別枠で、国外処理・保存の可能性を表示して明示有効化できる。国内未確認を理由に一律禁止しない」（P方針、暫定採用済み）と、`integration_grant`（初期は空・説明を読んだ scope 権限者による設定上の有効化・実I/Oは無効・監査）である [10章][pelway-ch10]。ADR-0008 の BYOK は同じ構造の先行例（テナント単位、初期オフ、管理者の明示的有効化、「国内限定の外。提供元での保存と学習への利用は契約者の責任」表示、有効化と取消を監査に記録）[ADR 0008][pelway-adr8]。

### (a) BYOK 的扱い（テナントの責任で有効化する例外） — 両立する

カタログ連携をこの例外枠で許可する使い方。10章の試験条件 D9（説明・社内有効化権限・提供根拠が揃えば設定上の有効化可。実I/Oの許可ではない）がそのまま当てはまり、技術上は両立する。LLM の BYOK と違う点が3つあり、表示と根拠の設計を広げる必要がある。

1. 資格情報そのもの（連携先の OAuth トークン）を国外のカタログ事業者に預ける。ADR-0008 の BYOK が預けるのは入力データで、Pelway 側の秘密は送らない建付けだったのに対し、カタログでは連携先の秘密の保管が事業者側に移る。
2. 経由地を通るのは本体の会話全体ではなく task の export 対象（検索語・文書ID を含む。10章は読み取りツールも無条件の無送信と見なさない）。
3. 学習への不使用は Composio（全プランで明記）と Pipedream（Connect の payload 不保存）で公式の述べがあるが、契約（DPA）単位での確認が別に要る。

表示文は「国内限定の外。資格情報とデータの米国経由を含む」の方向に拡張する。Rakazo の構成（カタログは運営者の設定時にのみ表示）も、カタログを opt-in にする同じ方向の先行例 [integrations][rakazo-integrations]。

### (b) 「国内で完結する連携のみカタログを使う」 — 今回の資料では成立しない

両カタログの実行経路自体が米国（Pipedream は us-east-1 を明記、Composio は未記載だが公表 IP が全て us-east-1）のため、「カタログを使いながら経路を国内に保つ」使い方は確認できなかった。読み替えて「国外配送をテナントが受け入れる連携のみカタログで出す」なら (a) の下位集合として成立する。

### (c) セルフホスト系 — 条件次第で参考になる

- Composio: Private VPC 配備・セルフホスト（Helm）・顧客管理鍵（AWS KMS／GCP KMS／Vault Transit）は Enterprise 契約。顧客管理鍵について公式は「秘密の保管を覆うが data residency は覆わない」と明記する。価格・提供条件は未確認 [token custody][composio-token-custody] [価格][composio-pricing-platform]。
- Pipedream: 企業向け AI ゲートウェイの Conduit はセルフホスト可（Docker／Kubernetes）。資格情報はインスタンス鍵（`CONDUIT_ENCRYPTION_KEY`）で AES-256-GCM 暗号化し自環境に置くが、「Pipedream アプリ」コネクタはホスト側（us-east-1）経由のまま。Connect 本体のセルフホストは今回の資料では確認できず [Conduit][pipedream-conduit-cloud] [connectors][pipedream-conduit-connectors]。

国内の自前環境に置けば資格情報と実行は国内に置ける見込みだが、契約・運用責任・部品（イメージ・更新）の海外依存は残る。MVP 後の調査候補とする。

## 推奨案と根拠

**MVPではカタログを採用せず自前実装とする。カタログ利用は (a) の BYOK 型例外枠で将来、個別に検討する（必要になった時点で別 Issue）。(b) は経路の実態と合わないため採らない。(c) は MVP 後の調査候補。**

1. **経由地域**: 両カタログとも米国経由。10章は検索語・文書ID も送信対象とみなすため、読み取り連携でも国外配送になる。国内完結（ADR-0008）の既定と両立せず、例外枠はあるが MVP の段階で例外経路を増やすと D4（経路逸脱試験）・D9（明示有効化）・D11（失効・取消）の試験面が広がる。
2. **段階宣言**: 両カタログとも操作の段階メタデータを持たない（未確認）。ADR-0004 は段階が決まっていない操作を取り消せない操作として止めるため、カタログの全ツールに Pelway 側の宣言と管理者確定がツール数ぶん必要になり、1,700〜3,000+ アプリ・10,000+ ツールという連携幅の利点を宣言運用が活かせない。自前なら検証対象の連携ぶんだけ宣言すればよい。
3. **資格情報**: 「モデルに渡さない」はカタログ・自前のどちらも満たす。差は保管の所在地（カタログは米国側、自前は国内 DB）で、ここは自前が優る。Composio の Self-managed credentials（保存しない実行時渡し）や顧客管理鍵で近づけられるが、いずれも Platform Enterprise 向けで条件が未確認。
4. **費用**: カタログ費用自体（Composio Pro $29/月〜、Pipedream Startup $99/月〜＋従量）は障壁にならない。MVP の連携数（2〜3個）では、自前の工数（技術文書§7 で段階3全体が3〜4週）とカタログ導入の作業（`integration_grant`・表示・段階宣言・試験の自前分）の差が小さい。連携数が数十に増える将来は再評価に値する。
5. **ロードマップ整合**: MVP は「外部への更新は承認を強制できる連携に限定」[roadmap](../roadmap.md)、「検証できない経路で外部更新を許可しません」[技術文書](../technical.md)。承認の強制点と操作面を Pelway が握る自前実装がこの前提を最も確実に満たす。

## 未確認事項

- Composio Cloud の処理・保存地域の公式記述。Trust Center（trust.composio.dev）は JavaScript 描画で本文を取得できず、サブプロセッサの所在地も未確認。
- 両社の日本リージョン提供の有無。
- Composio Enterprise（VPC 配備・セルフホスト・顧客管理鍵）と Pipedream Conduit セルフホストの価格・契約条件。
- 両カタログのツール定義に操作の段階（readOnly／destructive 等）のメタデータがあるか。API 仕様の全項目までは今回の確認で足りていない。
- Pipedream Connect 本体（資格情報管理と実行）のセルフホスト可否。
- DPA・BAA の条文内容（今回は公式文書の記載まで）。

## 出典

### Composio

- [Composio: Pricing（For You）](https://composio.dev/pricing) … Free／Standard $18/seat・月／Enterprise、Firewall・ZDR・BYOC の付帯
- [Composio: Platform Pricing](https://composio.dev/pricing/platform) … Hobby／Pro／Enterprise、従量単価、アドオン
- [Composio docs: Token custody architecture](https://docs.composio.dev/docs/security/token-custody) … 保管・マスク・実行経路・配備形式4種
- [Composio docs: Security](https://docs.composio.dev/docs/security/overview) … SOC 2 Type II、AES-256-GCM、組織分離
- [Composio docs: Data retention and handling](https://docs.composio.dev/docs/security/data-retention) … ログ保持1年・ZDR・学習不使用
- [Composio docs: Outbound IP addresses](https://docs.composio.dev/docs/security/outbound-ips) … 送信元 IP 4件
- [Composio: Enterprise](https://composio.dev/enterprise) … ISO 27001、セルフホストの位置づけ
- [Composio docs（SDK 全般）](https://docs.composio.dev/docs) … ツール定義の形（名前・説明・入力スキーマ）、scopes API
- [ComposioHQ/composio](https://github.com/ComposioHQ/composio) … MIT・SDK モノレポ

### Pipedream

- [Pipedream: Pricing](https://pipedream.com/pricing) … Free／Startup／Business
- [Pipedream docs: Privacy and Security](https://pipedream.com/docs/security/) … us-east-1 ホスト、KMS 暗号化、SOC 2・GDPR・HIPAA、Connect の payload 不保存
- [Pipedream docs: MCP Servers](https://pipedream.com/docs/connect/mcp) … 3,000+ アプリ・10,000+ ツール、資格情報のモデル非公開
- [Pipedream docs: Workflows Are Shutting Down](https://pipedream.com/docs/workflows) … 2027-03-31 終了（Connect は継続）
- [Pipedream docs: Conduit Cloud](https://pipedream.com/docs/conduit/cloud) … 無料枠・IP レンジ・セルフホスト案内
- [Pipedream docs: Conduit Connectors](https://pipedream.com/docs/conduit/configure/connectors) … コネクタ種別、資格情報の所在、インスタンス鍵
- [PipedreamHQ/pipedream](https://github.com/PipedreamHQ/pipedream) … コンポーネント registry

### その他

- [Rakazo docs: Connect apps](https://rakazo.com/docs/integrations/) … カタログはサーバー（運営者）設定依存、資格情報の暗号化とモデル非公開
- [Rakazo docs: Reusable credentials](https://rakazo.com/docs/bot-secrets/) … Postgres 内 AES-256-GCM、origin 一致の復号注入（自前実装の参考）
- [Amazon Web Services: AWS IP address ranges](https://ip-ranges.amazonaws.com/ip-ranges.json) … 公表 IP の us-east-1 割り当て確認に使用

### リンク定義

[composio-pricing-platform]: https://composio.dev/pricing/platform
[composio-token-custody]: https://docs.composio.dev/docs/security/token-custody
[composio-data-retention]: https://docs.composio.dev/docs/security/data-retention
[composio-outbound-ips]: https://docs.composio.dev/docs/security/outbound-ips
[composio-enterprise]: https://composio.dev/enterprise
[composio-docs]: https://docs.composio.dev/docs
[composio-repo]: https://github.com/ComposioHQ/composio
[pipedream-pricing]: https://pipedream.com/pricing
[pipedream-security]: https://pipedream.com/docs/security/
[pipedream-mcp]: https://pipedream.com/docs/connect/mcp
[pipedream-workflows-eol]: https://pipedream.com/docs/workflows
[pipedream-conduit-cloud]: https://pipedream.com/docs/conduit/cloud
[pipedream-conduit-connectors]: https://pipedream.com/docs/conduit/configure/connectors
[pipedream-repo]: https://github.com/PipedreamHQ/pipedream
[rakazo-integrations]: https://rakazo.com/docs/integrations/
[rakazo-secrets]: https://rakazo.com/docs/bot-secrets/
[pelway-ch10]: ../technical/v0.7/body/10-domestic-only.md
[pelway-adr4]: ../adr/0004-declared-action-tiers.md
[pelway-adr8]: ../adr/0008-llm-provider-and-byok.md
