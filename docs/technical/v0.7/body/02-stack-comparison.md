<a id="ch02-s01"></a>
# 02 技術比較と依存採用案

確定要件はRustベースのWindows/macOS/Linuxネイティブ、非WebViewを既定、ブラウザ対応、日英対応。外観を変えない。以下の採用は文書上の案で、組込み済み・試験済みを意味しない。

<a id="ch02-s02"></a>
## ネイティブUI

推奨はSlint、対照はegui。GPUIはWindows読み上げの阻害から初版の採用比較では保留する。UI担当と技術担当の案としてSlintを採用案とする（実機試験と固定版ライセンスの確認まで未確定）。公式issueの存在は、固定版で現在も再現する証拠ではないため、実機で再確認する。

<a id="ch02-s03"></a>
### 確認日と版

以下はUI担当の一次情報調査を統合した表。確認日は2026-11-08（日本時間）。


| 候補 | ライセンス | 最新リリース | 日本語入力（IME） | アクセシビリティ | 文字の選択 |
|---|---|---|---|---|---|
| [Slint](https://github.com/slint-ui/slint) | 3つから選べる。[Royalty-free 2.0](https://github.com/slint-ui/slint/blob/master/LICENSE.md)（非公開アプリも無料。ただしSlintを使っていることの表示が条件）／GPL-3.0-only／Slint Software 3.0（商用） | v1.18.1（2026-09-21） | デスクトップは対応。未解決の課題あり：[IME変換中にフォーカスが外れると最後の文字が消える #10861](https://github.com/slint-ui/slint/issues/10861) | AccessKitで対応。[大きいListViewでaccessibility機能の性能への影響が大きい #3867](https://github.com/slint-ui/slint/issues/3867)（未解決）。[仮想リストのフォーカス修正 #13425](https://github.com/slint-ui/slint/pull/13425)は済み | 未確認（複数のメッセージにまたがる選択の可否を実装前に確かめる） |
| [egui](https://github.com/emilk/egui) | Apache-2.0／MIT | 0.37.0（2026-10-15） | 0.35で改善（[変更履歴](https://github.com/emilk/egui/blob/master/CHANGELOG.md)）。未解決：[fcitx5で日本語IMEが動かない #2529](https://github.com/emilk/egui/issues/2529)、[X11＋fcitx5で変換中の文字が入力欄に出ない #7975](https://github.com/emilk/egui/issues/7975)、[同じ入力欄の中で変換が途切れる #8813](https://github.com/emilk/egui/issues/8813) | AccessKitを常に組み込み（[#7701](https://github.com/emilk/egui/pull/7701)） | ラベルの文字は選べる（[#3814](https://github.com/emilk/egui/pull/3814)）。部品をまたぐ選択は未確認 |
| [iced](https://github.com/iced-rs/iced) | MIT | 0.14.0（2025-12-07） | 0.14で対応（[#2777](https://github.com/iced-rs/iced/pull/2777)） | **未対応**。[Accessibility support #3281](https://github.com/iced-rs/iced/pull/3281)は取り込まれずに閉じた。[#552](https://github.com/iced-rs/iced/issues/552)は未解決 | [Text Selection #3497](https://github.com/iced-rs/iced/pull/3497)は2026-10-02に取り込み済みだが、リリース版にはまだ入っていない |
| [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)（Zedのフレームワーク） | Apache-2.0（[Cargo.toml](https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml)） | crateのversion 0.2.2。Zed本体と一緒に開発 | 対応しているが未解決が多い：[Windowsで変換中の文字の幅が違う #61724](https://github.com/zed-industries/zed/issues/61724)、[候補窓の位置ずれ #64850](https://github.com/zed-industries/zed/issues/64850)、[macOSの日本語IMEでメニューバーが壊れる #55835](https://github.com/zed-industries/zed/issues/55835) | [Windowsでスクリーンリーダーがまったく使えない #41138](https://github.com/zed-industries/zed/issues/41138)（2025-10から未解決） | エディタ部分は強い。チャットの表示に使えるかは未確認 |
| [Freya](https://github.com/marc2332/freya) | MIT | v0.5.0（2026-11-05） | 未確認 | READMEに改善の記述があるだけで、対応範囲は未確認 | 未確認 |

ほかに[Xilem](https://github.com/linebender/xilem)（Apache-2.0、v0.4.0）、[Blitz](https://github.com/DioxusLabs/blitz)（Apache-2.0、リリースなし）、[Makepad](https://github.com/makepad/makepad)（MIT、リリースなし）も見たが、リリースの状況から今回は比較の対象外にした。対象外にしたのは成熟度の判断で、性能の判断ではない。

<a id="ch02-s04"></a>
### Slintのライセンス：利用と配布の形ごと（一次資料：[LICENSE.md](https://github.com/slint-ui/slint/blob/master/LICENSE.md)、[Royalty-free License 2.0本文](https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)、[FAQ](https://github.com/slint-ui/slint/blob/master/FAQ.md)。masterブランチの条文、2026-11-08確認。リリース版v1.18.1に同梱の条文と同じかは未確認）

| 利用・配布の形 | Royalty-free 2.0 | GPL-3.0-only | 商用 |
|---|---|---|---|
| Pelwayのデスクトップ版を非公開のソースで配る（Windows/macOS/Linux） | 使える。ただし次のどちらかが条件：(a) アプリの最上位メニューから開ける「About」画面に`AboutSlint`の部品を出す（About画面がなければ起動画面に出す）、(b) 配布ページなどの公開Webページに「Made with Slint」の表示を置く | 使えない（アプリ全体をGPL互換で公開する場合だけ使える） | 使える（有料。価格は[料金ページ](https://slint.dev/pricing)、今回は確認も購入もしない） |
| Pelwayのデスクトップ版をオープンソースで公開する | 使える | 使える。自分のコードはMITなどGPL互換のままでよいが、組み合わせた全体はGPLになる | 使える |
| Web版にSlintを使う | 条文上は「Web Application」として使える。ただしUI担当の推奨ではWeb版にSlintは使わない（Web版の比較を参照） | — | — |
| 組み込み機器（専用端末など） | 使えない | 使える | 使える |
| Slintを単体で配る／アプリからSlintのAPIを外に出す（例：利用者やBotがSlintのAPIで画面を拡張できるプラグイン） | 使えない（第3条） | GPLの条件で可 | 契約次第（未確認） |

UI担当の判断：Pelwayの初期のデスクトップ版では、Royalty-freeの(a)か(b)を満たせば費用なしで使える見込み。ただし、画面の拡張をプラグインとして外に開く設計にすると第3条に触れる恐れがあるので、その設計は避けるか商用ライセンスを検討する。最終的な判断は濱田さんで、法的な確認は情報セキュリティ・法務の担当に回す。外観については、About画面に`AboutSlint`の部品を置くことは画面の構成に関わるので、置く場所はUI担当が決める。


<a id="ch02-s05"></a>
### チャット要件での評価

| 観点 | Slint | egui | その他候補と判断 |
|---|---|---|---|
| 長文・大量履歴 | ListView等を利用し可変高さ・読んでいる位置の保持・読み上げを検証。大規模リストの課題あり | 表示範囲と長文選択を実装・検証 | GPUIはエディタ実績をチャット全体の保証にしない |
| 和英混在・IME | 3OSの変換中の文字、フォーカス移動、候補窓の位置を試す | Linux fcitx5課題を先に試す | icedにもIME実装はあるが読み上げ条件が不足 |
| 選択・コピー | 複数messageに跨がる選択は未確認 | 同じく未確認 | OS clipboard連携だけでは要件合格ではない |
| アクセシビリティ | AccessKitを使いNVDA/VoiceOver/Orcaで合格確認 | AccessKitを使い同条件比較 | icedとGPUIは初版条件に対して阻害あり。対象版の再確認が必要 |
| 通知 | framework外のhost adapterでローカル通知 | 同じ | Windows/macOS/Linuxのpermission・focus復帰・本文漏えいを別試験。APNs/FCMは初期不使用 |
| 保守・配布 | 上表のreleaseとlicense条文を固定tagで再照合 | MIT/Apacheと同梱fontの条文を照合 | 保守頻度や「対応」宣言はSLA・実機保証ではない |

SlintのRoyalty-freeは標準的なpermissive OSS licenseと同じとは扱わない。AboutSlint等の条件、APIを外へ公開する制限を固定版で確認する。外部AI連携のAdapterはUI拡張APIと分離し、Slint APIを外部へ公開しない。技術的に分離しただけで法的適合を断定しない。

<a id="ch02-s06"></a>
## ブラウザ版と共通核

| 案 | 長所 | 負担 | 判断 |
|---|---|---|---|
| TS＋React DOM UI、Rust同期核をWASM | browserのIME・読み上げ・文字選択を使う。状態・同期契約を共有 | UIは二重実装。WASM bridgeと保存adapterが要る | 推奨 |
| Slint/eguiをWASMで共通UI化 | 画面コード共有の可能性 | canvas系の表示とブラウザの検索・選択・読み上げの整合を別途検証 | 初版では採らない |
| Rust DOM UI（Leptos等） | Rustで記述統一 | ネイティブ部品は結局別。新しいUI基盤の評価が増える | 初版では採らない |
| Webの同期まで全部TS | WASM bridge不要 | 同期規則の実装が二つになる | WASM費用が大きい場合の代替 |

共有するのはdomain enum、API型、同期・dedup、翻訳資源、表示規則、デザイントークン。共有しないのは画面部品、focus、IME、OS通知、clipboard、ファイル選択。承認規則を共通核で表示しても、最終判定はserverで強制する。

<a id="ch02-s07"></a>
### 二重UI実装の部分工数


UI担当の[input/ui.md](../sources/v0.6/fixed/pelway-technical-docs/input/ui.md) §3a・§5に基づく粗い仮定。考慮したのは部品ごとの状態と、[07の比較の順番・負荷と記録](07-performance-plan.md)の試験項目で、定量的な係数/実績で裏付けた見積もりではない。実装する人の人時で、botの作業時間は含めない。短期・中期の人時には足さず、別枠として扱う。実際の値は、07の確認試験で1つ目の部品を作ったあとに見直す。

| 部品 | Web版 | ネイティブ版 | ネイティブ版が重い理由 |
|---|---|---|---|
| メッセージの一覧（仮想スクロール） | 8〜16h | 16〜32h | 行の高さがばらばらな仮想リストと、読み上げとの両立を自前で確かめる |
| メッセージ入力欄（日本語入力・メンション候補） | 6〜12h | 12〜24h | 3つのOSの日本語入力の試験が要る |
| 承認の画面と状態の表示 | 6〜10h | 8〜14h | 状態の変化を読み上げに伝える仕組みをOSごとに確かめる |
| 判断待ちの一覧 | 4〜8h | 6〜10h | キーボード操作の試験 |
| 「作業を見る」画面（静止画と手順のログ） | 4〜8h | 6〜12h | 画像の更新と読み上げの両立 |
| 通知 | 2〜4h | 6〜12h | OSごとの通知の仕組み（技術担当の選定次第） |
| 設定と言語の切り替え | 2〜4h | 4〜8h | 再起動なしの切り替え |
| **合計** | **32〜62h** | **58〜112h** | |

この表に入っていないもの（未見積もり。0時間とは扱わない）：07の確認試験の準備と実施、試験後の修正、デザインの値と文言を1か所にまとめる仕組みの準備。


外部連携の表示、bridge、service Adapter、国内境界の確認、OS署名・配布は上表に含めない。合計90〜174人時は2列を足した部分工数であり、総開発工数ではない。通知の参考値を含み、実績・実装担当者の技能・再利用可能部品を確認していない。短期v0.1や中期v0.4の人時に自動で加算しない。Web先行か同時進行かの順番も今回決定しない。

<a id="ch02-s08"></a>
## サーバー・AI層の比較

| 用途 | 推奨案 | 代替と採用理由 |
|---|---|---|
| 非同期/HTTP | tokio、axum、tower | actix-webも比較可能。Rust共通型とmiddlewareで認証・traceを統一する案 |
| DB | PostgreSQL＋sqlx | SQLite serverは部品が少ないが並行処理と将来の会社分離を別検証。今回は既存Postgres方針を維持 |
| client保存 | rusqlite / IndexedDB adapter | SQLiteをbrowserへ無理に同じ実装で持ち込まない |
| full text | Postgresの権限付き検索を入口とし、PGroonga（TokenBigram＋NormalizerNFKC）を第一候補とする | pg_bigmと合成データで比べてから正式に決める。Tantivy/LinderaはAI検索の補助indexが必要になったときの将来候補。[調査](../../../research/japanese-full-text-search.md) |
| RAG | 初版必須ではない。必要時に国内モデル＋pgvectorを比較 | wiki/Q&A自動生成は範囲外。外部embeddingへ自動接続しない |
| MCP | rmcpを条件付き依存採用案 | protocol実装を流用し、業務承認やsandboxはPelway側 |
| LLM adapter | llama.cppの事前取得modelを第一候補、rigは必要なら小範囲 | LangGraph/OpenHands/Gooseは参考。全runtimeを組み込まない |
| 観測 | tracingのローカル出力。承認・開始資格・結果照会の各段階に同じtrace idを最初から付ける | OpenTelemetryは将来の相互運用候補。外部exporterや自動設定を初期有効にしない |

<a id="調査を受けた採否"></a>
## 調査を受けた採否

[技術調査](../../../research/README.md)と[競合調査](../../../competitors.md)を受けた採否。「MVP前に検証」は採用の決定ではなく、合成データ・局所試作で確かめてから採否を決める項目。

| 領域 | 採否 | 内容 |
|---|---|---|
| 非同期実行 | 採用：DBキュー（自作SKIP LOCKED+lease） | `FOR UPDATE SKIP LOCKED`とleaseの定番パターンに限る。止まったworkflowの再開は状態tableから判断する。Temporal・Restateは採らない。River（`riverqueue`）と比較した結果、自作を採りRiverは将来の候補（[ADR 0006](../../../adr/0006-db-queue-choice.md)） |
| server間の通知 | 採用：`LISTEN/NOTIFY` | 起こす合図だけに使い、正本は`seq`。合図を取りこぼしても定期の追いつき検索で補う（[ADR 0006](../../../adr/0006-db-queue-choice.md)で実証）。Redisは入れない |
| テナント分離 | 採用：Row Level Security | `workspace_id`の複合外部キーとアプリの絞り込みに加える二重の守り。[ADR 0005](../../../adr/0005-postgres-row-level-security.md) |
| 全文検索 | MVP前に検証 | PGroonga第一候補、pg_bigmと比較。WAL・復元、rootless Podman、英字/コードの部分一致、表記ゆれを試す |
| サンドボックス | MVP前に検証 | gVisor（runsc）とcrun＋libkrunを、rootless Podmanとの組み合わせ・起動時間・syscall互換・egress拒否の維持で測る |
| 添付の保存 | 採用：`object_store`経由 | 保存先はローカルのまま、S3互換interfaceで扱う |
| ローカルLLM・埋め込み | MVP前に検証 | llama.cpp＋Apache-2.0の日本語model（LLM-jp-4、Qwen3-Swallow）、ruri-v3。各modelのLICENSE全文を確認する |
| MCP | MVP前に検証 | rmcp。tool annotationsは信頼しない（[ADR 0004](../../../adr/0004-declared-action-tiers.md)） |
| 認証 | MVP前に検証 | webauthn-rs（passkey）とopenidconnect-rs（顧客IdP）をaxumに組み込む。Ory Kratos/Hydraは将来候補。Keto・OpenFGA・SpiceDBは12の外部権限サービス不要の方針により採らない |
| クライアント | MVP前に検証 | A1にTauri＋React（Webと画面共有）を比較候補として加える。Slint issue #3867・#10861の再現、ts-rsまたはspecta、wasm-opt |
| バックアップ | MVP前に検証 | pgBackRest。保存期間は件数指定か別の削除処理で14日上限を守る。2.59.3以上 |
| 監査 | MVP前に検証 | `prev_hash`の鎖を残し、最新hashと行数に署名したcheckpointをDB管理者と別の場所へ保存する。形式と鍵・保管先の分離は[ADR 0007](../../../adr/0007-signed-audit-checkpoints.md)、下書き実装`crates/pelway-audit-checkpoint`で改ざん検出を検証済み |
| 外部エージェント | 採らない（MVP） | ACPによるClaude Code・Codex等の参加は国内完結の前提と衝突する。選択外部連携の枠で将来候補 |
| 将来の候補 | 採否は将来 | OpenBao、Cedar、Ory Kratos、LiveKit、OpenMLS、YomiToku（商用ライセンスが必要）、NDLOCR-Lite、PaddleOCR、pgvector、OpenTelemetry Collector |

直接依存の許諾は[09の対応表](09-sources.md)と[技術照合記録](../sources/v0.6/fixed/pelway-technical-docs/reviews/technical-license-ledger-v0.1.md)に示す（2026-11-08、可変ブランチ/最新版の観測値であり採用版ではない）。rmcpはApache-2.0への移行中でMIT部分が残る。全推移依存・NOTICE・SBOMは未作成、固定版の条文/脆弱性確認も未了。適切なOSSの採用は進めるが、安全境界をOSSの既定動作へ任せない。

<a id="ch02-s09"></a>
## Discordの事例

[Discord CTO本人の投稿](https://x.com/svishnevskiy/status/2118081920194203799)は保存済み公開投稿の取得記録で確認した（今回の差分レビューで新規取得したものではない）。2026-11-05 05:44（日本時間）の投稿は、Electron/React NativeからRustへの移行方針とGPUI採用を説明し、ユーザー試験は今後とする。移行完了や一般配布まで確認したとはしない。一方、「1.3GB→300MB」の同条件のメモリ比較は未検証。移行方針と数値の確からしさを分け、どちらもPelwayのUI採用根拠や性能保証へ転用しない。取得証拠はreviews/discord-primary-post-evidence.txt。

[市場調査の一次資料と未確認範囲](../sources/v0.6/fixed/pelway-technical-docs/research/existing-chat-and-agent-oss.md) を参照。Rust・非WebViewはユーザー要件として維持し、自分たちの07の測定で判断する。
