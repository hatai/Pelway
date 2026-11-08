<a id="ch01-s01"></a>
# 01 ローカル構成と責任分界

この章は構成の説明。国内に物理設置された管理端末で動くローカル構成を設計する。設置場所と管理状態は未確認。クラウド本番を構築しない。

<a id="ch01-s02"></a>
## 全体図

```mermaid
flowchart LR
 subgraph JP[日本国内設置ホスト・管理ネットワーク：未検証の前提]
  GUI[Rust + Slint GUI]
  WEB[ブラウザ DOM UI]
  CORE[Rust client-core / WASM]
  subgraph APP[ローカル本体・外向き既定拒否]
   API[server: axum / 認証と承認]
   DB[(PostgreSQL / outbox / 監査)]
   FILE[(添付・成果物・Botメモリ・バックアップ)]
   RUN[runner: リース / 手順再開]
   LOCAL[LLM gateway: スタブ / ローカル推論]
   GW[連携gateway: 権限 / 秘密 / 通信policy]
   CTRL[隔離されたsandbox起動controller]
   SB[タスクsandbox: 秘密なし / 外部出口なし]
   AD[信頼側Adapter: 型付き要求 / secretなし]
  end
 end
 subgraph CLIENT[任意の利用者管理端末：所在・接続条件は別審査]
  BR[local bridge / CLI / hook：将来案]
 end
 subgraph EXT[利用者が明示選択する外部サービス：国外処理・保存あり得る]
  AG[Claude Code / Cursor等]
  TOOL[Datadog / Google Workspace等]
 end
 GUI --> CORE
 WEB --> CORE
 CORE -->|loopback HTTP/WS（合成データ試験）| API
 API --> DB
 API --> FILE
 RUN --> DB
 RUN --> LOCAL
 RUN --> CTRL
 CTRL --> SB
 SB -->|操作要求のみ| GW
 GW -->|内部ファイル操作| FILE
 GW -->|承認済み固定入力| AD
 AD -->|型付き要求: 外部出口はGWだけ| GW
 GW -->|予約/直前再検査/attempt記録| API
 GW -.->|将来の認証接続：方式未決・初期未実装| BR
 BR -.-> AG
 GW -.->|許可済みtransportだけ| AG
 GW -.->|許可済みtransportだけ| TOOL
 AG -.->|進捗 / 差分 / 失敗：未信頼データ| GW
 TOOL -.->|文書 / アラート：未信頼データ| GW
 GW -->|帰属/署名検証してinboxへ保存| API
```

実線の本体データは国内ローカル保存・処理の設計。破線は利用者選択の例外経路で、本体の「国外送信なし」に含めて保証しない。外部から取り込んだ成果物のPelway保存コピーは国内に置く。提供元に残るコピー・ログ・セッションは外部サービスの責任分界。local bridgeは利用者端末で動く案で、管理host内に固定しない。ただし国外端末への接続許可を得た意味ではなく、所在/認証/端末管理を別に確認する。

<a id="ch01-s03"></a>
## ホストとコンテナ

| 部品 | 実行 | ビルド・試験 |
|---|---|---|
| ネイティブGUI | 各ホストの通常プロセス | Windows MSVC、macOSのSDK、Linuxのネイティブ環境。IME、GPU、通知、読み上げはホストで試す |
| Web UIとWASM | ホストブラウザ | 静的資源のビルドはコンテナ可。ブラウザ機能の試験はホスト |
| server、runner、gateway | Linuxコンテナ。論理境界として別プロセス | Linux向けイメージ。最低3プロセスであり「2プロセス」とは呼ばない |
| sandbox controller | 信頼するホスト/コンテナランタイム側の限定API | 任意のruntime socketをBotやsandboxへ公開しない |
| DB、ファイル、推論 | Linux側。GPU推論がホストAPIを要する場合は国内ホストの別プロセス | CPU別イメージとdriverを検証。macOSのGPUをLinuxコンテナから当然使えるとはしない |
| サンドボックス | 同一ランタイムの独立したコンテナ | 入れ子コンテナを既定にしない。ホストホームやruntime socketをマウントしない |

WindowsのWSL 2とmacOSのApple containerはLinux実行基盤。Windows/macOS GUIの製品試験や署名を代替しない。GUIからの入口はloopback限定。localhostでもOrigin検査、CSRF対策、認証を省略しない。

<a id="ch01-s04"></a>
## 仕事の往復

```mermaid
sequenceDiagram
 participant H as 人
 participant S as Pelway server
 participant R as runner
 participant G as 連携gateway
 participant E as 選択済み外部agent
 H->>S: メンションと依頼
 S->>R: taskとrunを永続化して開始
 R->>G: scopeと送信payloadを指定
 G->>S: 固定payloadと必要承認の提示
 S-->>H: 送信先・データ・国外可能性・権限の確認
 H->>S: 当該操作を承認
 S-->>G: 予約済operation/attempt/payloadへの一回限りの開始資格を発行
 G->>S: 発行から5秒以内に当該資格で最終検査と開始受理を要求
 S->>S: 検査と資格消費・条件付き開始更新を同一transactionで永続commit
 S-->>G: 当該gateway実行だけに開始受理の成功応答
 Note over S,G: commit/応答不明・重複・再起動では呼び出さず照合
 G->>E: run/operation ID付きで仕事を渡す
 E-->>S: 進捗・成果物・差分・失敗をinbox経由で返す
 S-->>H: 判断待ちと未確認成果物
 H->>S: 確認 / 修正依頼 / 再開 / 取消（受付） / 引継ぎ
```

Adapter未実装の初期ローカル検証では、外部agentを疑似サービスに置き換える。外部サービス選択の許可は、現在の実接続・実送信の承認ではない。

<a id="ch01-s05"></a>
## 永続化と正しさ

業務の正本はserverとPostgreSQL。状態変更、監査、配信用outboxは同じDBトランザクションで確定する。WebSocketは通知経路であり正本ではない。DBキューを使い、ローカル初版へ別のメッセージブローカーを持ち込まない。

Botの人格、役割、メモリと、実行コンテナの寿命を分ける。ファイルとチェックポイントは国内の管理領域に残す。担当交代で他Botのボリューム全体を渡さず、taskに結び付く検査済み成果物・手順だけを権限付き参照で引き継ぐ。

配置先の共通境界はPlacementPolicyとし、04/10に定義する。今は日本local単一で、将来のUS/EU/JPは選択肢の例示だけ。外部連携4製品も例示であり限定しない。

初期のloopback構成は、単一host上の複数アカウントによる合成データ検証専用。別端末の実メンバーが共同利用できる完成品ではない。LAN公開・TLS・端末認証・具体的ローカル認証方式は未決とし、localhostというだけで認証を省かない。APIの承認検査/attempt記録をgatewayからserverへ要求し、成功後だけ外部transportを呼ぶ。

<a id="ch01-s06"></a>
## 全構成に適用する情報取扱い制御

server/runner/推論/gateway/公開/検索/運営/復旧は[12](12-communication-secrecy.md)の現権限・目的・根拠とsource制限を共有する。保存・quota・期限・有人枠は[13](13-operational-defaults.md)の選定値に従う。P方針は内部開始受理を境界とする取消契約の暫定変更であり、S1の技術的解消や実I/O許可ではない。未決の正本は[08](08-decisions-open.md)だけとする。

明示メモリは作成/根拠を確認した明示更新から最長365日、全source期限以内。閲覧で延長せず、毎回全件をAIに入力しない。必要範囲だけを取得し、将来の索引/RAGを初期必須機能には戻さない。
