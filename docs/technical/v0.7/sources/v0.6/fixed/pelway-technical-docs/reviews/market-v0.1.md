# 市場レビュー：Pelway技術文書 v0.1

確認日：2026-11-08（日本時間）。市場・既存チャット/AI OSS調査担当による文書レビュー。
対象は `v0.1-snapshot/`。02・08・09・11を重点確認し、残りの固定本文も出典・数字・責任分界について通読した。
照合原稿は `../research/existing-chat-and-agent-oss.md`。追加指示に従い `technical-v0.1.md` のTR-3と `discord-primary-post-evidence.txt` も確認した。
本文・固定版・原稿は変更していない。実装、接続設定、登録、公開、WSL操作、Git操作は行っていない。外部アクセスは公開情報の読取りのみ。

## 賛否

**条件付き賛成。** 設計の方向と責任分界には賛成。M-01〜M-03を文書上で解消し、M-04の参照を直してから固定版を更新する提案。
本レビューは採用確定、ライセンス適合、国内限定適合、実機合格、実データ利用の承認ではない。

- [確認事実] rmcp・rig・llama.cppを無条件の採用確定へ格上げした記述は確認しなかった。02:3、91〜95、08:3、15が条件を保持する。
- [確認事実] LangGraph/OpenHands/Gooseは02:92で参考。Matrix/Zulipは09:21と04:58で参考であり、製品コードの流用を指示していない。
- [確認事実] README:24〜28、10:33〜37、11:29は本体国内限定と利用者選択連携を分ける。外部連携は初期オフ、説明後に権限者が明示有効化し、個別承認・監査・秘密管理を維持する。
- [確認事実] 4製品は例示で限定しない旨はREADME:34、11:3にある。将来リージョンが法適合を保証しない旨は08:18、10:72にある。
- [確認事実] 02:7、24、41〜46、07:3〜7は公式対応/issueと対象版の実機試験を分ける。03:90はモデル名・hash・許諾の固定を未実施の条件としている。

## 要修正

重大度は「高＝設計や責任分界の誤り」「中＝事実・採用条件・追跡可能性の修正」「低＝参照や表現の補正」。確認した指摘は中3件、低1件。高に該当する確定指摘はない。

### M-01（中）：Discordの移行方針・GPUI採用の一次情報を追加。市場原稿の自己訂正

- **箇所**：固定版 `02-stack-comparison.md:97〜101`、`09-sources.md:26〜28`。照合原稿 `research/existing-chat-and-agent-oss.md:55〜61`、特に60行。技術レビュー `technical-v0.1.md:13` のTR-3。
- **確認事実**：指定されたCTO投稿を公開X読取りで直接取得し、長文本文 `note_tweet.text` を確認した。保存済み証拠の3行目とも一致する。投稿・役職・日付・文言・対象範囲の詳細は次節。
- **原稿との差分**：原稿60行の「対象製品・移行範囲…の一次資料は確認できない」は、今回確認した移行方針・GPUI採用まで一括して未確認に読めるため訂正が必要。[D0]のGo→Rust記事だけが一次資料ではなく、別の新しい一次投稿がある。旧記事の説明自体を撤回する必要はない。
- **TR-3への判断**：「移行の事実は一次情報あり、数値は未確認」は、**事実を『移行方針の表明とGPUI採用』に限定すれば賛成**。「未確認なのは数値だけ」は広すぎる。移行完了・一般配布も当該投稿では確認できない。
- **修正案**：02の旧下書きに対する伝聞を、以下の一次情報による説明へ置き換える。09には投稿URLと役職確認URLを追加する。原稿55〜61行の訂正は次回編集時の差分とし、今回は原稿を変更しない。

> [事実] Discord CTO兼共同創業者Stanislav Vishnevskiyは、2026-11-05 05:44:16（日本時間）の公開投稿で、クライアントをRustへ移行し、Electron/React Nativeを置き換える方針とGPUIの採用を述べた。モバイルはnative UIを保ち、Rustのアプリコードをデスクトップと共有するとしている。ユーザー試験はこれから行いたいとの記述であり、移行完了・一般配布済みはこの投稿から確認できない。「1.3GB→300MB」の同条件比較は未検証。いずれもPelwayの採用理由・性能目標・Rust非WebView条件の根拠には使わない。

- **GPUIの判断は維持**：Discordの採用は、Pelwayで必要なWindows読み上げの成立を証明しない。zed#41138による阻害と、02:7・19、07:7の初版主候補からの除外を取り消す理由にはならない。issueにはAccessKit作業の進捗もあるため、「あらゆる版のGPUIで永久に不可能」とも書かない。対象版のNVDA等の実機試験は未実施。

### M-02（中）：固定版内から詳細出典へ進む相対リンクが切れている

- **箇所**：`09-sources.md:7〜10,24`、`02-stack-comparison.md:101`、`11-integrations.md:56`。同種は `README.md:50,52`、`04-data-api-realtime.md:58`、`08-decisions-open.md:45`。
- **確認事実**：固定版09の `research/existing-chat-and-agent-oss.md` は `reviews/v0.1-snapshot/research/...` に解決され、存在しない。`input/*.md` と `status.md` も同様。技術文書ルートを基準にすれば対象は全て存在し、ルートに置く本文のリンクの誤りではない。
- **影響**：レビュー用固定版を単体で開くと、ライセンス・通信・E1〜E19の詳細な出典へたどれない。manifestの対象は本文12ファイルであり、参照原稿の内容までは固定されていない。
- **根拠**：対象ファイルのリンク文字列とfilesystemの存在確認。manifestにはresearch/input/statusのエントリがない。
- **修正案**：今回の固定版はそのまま残す。次版で、固定版と対応する出典原稿のhashを別の出典manifestに記録し、固定版から到達できる出典索引を用意する。現行原稿へつなぐ場合は `../../research/...` 等に解決する索引を使い、リンク先が可変であることと照合hashを明示する。既存manifest一致を失うような今回の固定版の直接修正は行わない。

### M-03（中）：新規の推奨依存とライセンス台帳の対応が欠ける

- **箇所**：`02-stack-comparison.md:84〜95`、`09-sources.md:5〜24`。
- **確認事実**：02:95は「用途・ライセンスの一次資料は09と市場調査に集約」とするが、09にはtokio/axum/tower、sqlx/PostgreSQL、rusqlite、React等の各採用単位の許諾・公式LICENSEへの対応がない。市場原稿の「ライセンスと保守の確認台帳」は既存チャット/agent/UI等が対象で、この新しい直接依存一式の許諾台帳ではない。
- **影響**：「条件付き依存採用案」という分類自体は正しいが、組込み対象と許諾根拠の対応を読者が確認できない。MIT/Apache系と推測して埋めたり、参考製品の許諾を依存へ流用したりしてはならない。
- **根拠**：市場原稿§「ライセンスと保守の確認台帳」「推奨区分・差分と未確認事項」、固定版09の一次資料表。
- **修正案**：02/09に、少なくとも直接依存ごとの「参考／条件付き依存採用候補／改変・流用候補、対象crate/配布物・版、公式LICENSE、同梱物/NOTICE、未確認条件」の対応を追加する。未調査は未確認と記し、02:95を「一部は台帳未作成」に直す。全推移依存の確定はlock/SBOM段階の別作業とし、今回済んだことにしない。

### M-04（低）：工数の根拠で参照する「§5」が統合先で不明瞭

- **箇所**：`02-stack-comparison.md:64,77`。
- **確認事実**：「§5の試験項目」「§5の確認試験」とあるが、02に対応する番号付き§5はない。元の `input/ui.md` では§5が「実装前の確認試験」であり、05の開発環境章を指すものではない。
- **根拠**：`input/ui.md:47〜62,78`。32〜62h、58〜112hはこのUI担当原稿の粗い仮定で、外部製品の実績値ではない。02の各列の合計と90〜174人時の加算は一致する。
- **修正案**：「UI入力の§3a・§5に基づく担当者の粗い仮定。実績なし」と具体的に参照するか、統合先07の入力/履歴/読み上げ試験へのリンクに置き換える。状態数から人時への算定係数は示されていないため、定量モデルで裏付け済みとは書かない。既存計画へ加算しない扱いは維持する。

## Discord TR-3の一次情報確認記録

| 確認項目 | 確認事実と限界 |
|---|---|
| 投稿の実在 | 公開X読取りのpost ID `2118081920194203799` を直接取得。その後の公開検索で長文 `note_tweet.text` も取得。保存証拠 `discord-primary-post-evidence.txt:3` と同じ本文。引用された非公式Discord Previewsの説明を、CTO本人の本文と混ぜていない。 |
| 投稿者 | author ID `22238139`、`@svishnevskiy`、表示名Stanislav、プロフィールはDiscord CTO & Co-Founder。保存証拠にはDiscordへの所属情報もある。さらにDiscord公式companyページが「Stanislav Vishnevskiy, CTO」と明記。認証バッジだけを本人・役職の根拠にしていない。 |
| 投稿日時 | APIの日時を換算し **2026-11-05 05:44:16（日本時間）**。技術レビューの11月4日という日付表記とはタイムゾーンが異なる。 |
| 正確な文言 | “That means Electron goes away, and so does React Native, mobile keeps native UI but shares app code with desktop via Rust.” “We hope to get it into testing with users soon™.” “And it's not Tauri, it's GPUI” を長文本文で確認。 |
| 対象 | 「whole client」、デスクトップとモバイルの共有アプリコードを対象にした移行方針。モバイルのUI自体をGPUIへ置換するとは読めず、native UI保持の明記がある。単なるRead Statesサービス移行の話ではない。 |
| 完了・配布 | 当該投稿はユーザー試験をこれから行いたいとする。移行完了、一般配布済み、全OSでの本番品質は確認できない。「移行方針・GPUI採用の表明がある」と「移行完了」を分ける。 |
| メモリ数値 | 当該投稿の本文に1.3GB→300MBの同条件比較はない。添付画像の単一時点の値があっても、旧版との対象・操作・負荷・プロセス範囲が一致した比較の証明にはならない。数値・改善率は未検証のまま。 |
| 選定への影響 | Pelwayの採用理由には不使用。GPUIの読み上げ阻害による保留を維持。Discordの体験、IME/a11y、性能をPelwayの合格へ外挿しない。 |

一次資料：
- CTO投稿：https://x.com/svishnevskiy/status/2118081920194203799
- 役職の公式確認：https://discord.com/company/
- GPUI関連の報告と保守者コメント：https://github.com/zed-industries/zed/issues/41138
- 別件であるRead States移行：https://discord.com/blog/why-discord-is-switching-from-go-to-rust

WebFetchではX投稿が403だったが、公開X読取りで本文を確認できた。curlは同URLで200。HTTP到達性と本文確認を区別し、403を投稿不在の証拠にしていない。二次報道は発見補助に留め、上記結論の根拠には採用しない。

## ライセンス・分類・数字の確認結果

- [確認事実] Slint Royalty-free 2.0本文はdesktop/mobile/webの定義、帰属表示の選択肢、Slint単体提供・embedded利用・API公開の制限を定める。固定版02:24〜34,48はその区別とmaster/配布版の未照合を保持する。標準permissive OSSと扱わない点にも賛成。実際の配布版の法的判断は未了。
- [確認事実] 原稿はSynapse/Element/MattermostのAGPL/GPL等とMCP/モデル/配布物の別許諾を区別する。固定版ではこれら製品のコード採用・GPL/AGPLコードの無条件流用へ変更していない。製品名だけで「すべてMIT」等とする誤記は対象本文で確認しなかった。
- [確認事実] llama.cppの許諾をモデル重みへ自動適用する記述はなく、モデルは03:90と08:15,35〜37で個別確認待ち。Qwen等の原稿中の候補を、そのまま採用モデルに格上げしていない。
- [確認事実] 02の部分工数、07の負荷・メモリ・描画目標、04:56のbackoff上限は仮定と明記される。300MiB目標は07:22でDiscordを根拠にしないと明記。出典のない競合実測値や改善率として転用した箇所は確認しなかった。
- [未確認] 全license本文・全推移依存・全リリース日を今回再調査したわけではない。M-03と固定版照合が残るため「全ライセンス検証済み」とは判定しない。

Slint一次資料：https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md

## 出典リンク・IDとハッシュの検査

### ローカル参照とID

- [確認事実] manifest掲載12ファイルは全て存在し、各SHA-256が一致。余分な固定版ファイルはなかった。不一致0。
- manifestファイル自身のSHA-256：`27a23b4fc2acaa28f726718294b03ec910aef06fb82803dfb9056708bbf14612`。
- 照合時点の市場原稿SHA-256：`5fe16ed4a92a3633dbc60bd940f076e0b71474af82eb35071adfbd502512511e`。この原稿hashはv0.1-manifestの収録対象ではなく、本レビューで別記したもの。
- [確認事実] 原稿のURL参照定義は116件。参照IDの未定義は0、09:24で挙げるE1〜E19は全て存在する。固定本文は通常のMarkdownリンクを使い、原稿の参照定義を自動継承しているとは書いていない。
- [確認事実] 固定本文同士の相対ファイルリンクは存在する。外部のresearch/input/statusへの相対リンクの問題はM-02。見出しアンカーの網羅検査ではない。

### 主要URLのcurl確認

`curl --location --silent --show-error --max-time 15 --connect-timeout 5 --output /dev/null --write-out '%{http_code}\t%{url_effective}' URL` によるGET。下記21件は全て終了コード0・最終HTTP 200。404は0、403/timeoutもこのcurl実行では0。200は本文の正しさ・ライセンス適合の証明ではない。未掲載リンクは今回の疎通確認対象外。

| 一次資料URL | 最終HTTP |
|---|---|
| https://github.com/slint-ui/slint/blob/master/LICENSE.md | 200 |
| https://github.com/slint-ui/slint/blob/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md | 200 |
| https://github.com/slint-ui/slint/issues/3867 | 200 |
| https://github.com/iced-rs/iced/pull/3281 | 200 |
| https://github.com/zed-industries/zed/issues/41138 | 200 |
| https://github.com/modelcontextprotocol/rust-sdk/ | 200 |
| https://github.com/ggml-org/llama.cpp/blob/46dba9fc/tools/server/README.md | 200 |
| https://docs.vllm.ai/en/stable/usage/usage_stats/ | 200 |
| https://spec.matrix.org/latest/client-server-api/ | 200 |
| https://zulip.com/api/get-events | 200 |
| https://discord.com/blog/why-discord-is-switching-from-go-to-rust | 200 |
| https://code.claude.com/docs/en/headless | 200 |
| https://code.claude.com/docs/en/hooks | 200 |
| https://cursor.com/docs/cloud-agent/api/endpoints | 200 |
| https://docs.datadoghq.com/mcp_server.md | 200 |
| https://docs.datadoghq.com/getting_started/site.md | 200 |
| https://developers.google.com/workspace/guides/configure-mcp-servers | 200 |
| https://knowledge.workspace.google.com/admin/compliance/choose-a-geographic-location-for-your-data | 200 |
| https://github.com/apple/container/blob/main/BUILDING.md | 200 |
| https://x.com/svishnevskiy/status/2118081920194203799 | 200 |
| https://discord.com/company/ | 200 |

## 実装より前に決めるべき未決事項

以下は設計提案であり、新しい採用決定や実行許可ではない。

1. **依存と許諾の単位**：Slintの固定版・配布形態・帰属表示、rmcpの固定版許諾範囲、M-03の直接依存台帳。Goose等を参考から改変・流用へ進めるなら対象ファイル・依存・NOTICEを別承認する。
2. **推論モデルと機材**：重み/量子化/トークナイザーの版・hash・許諾・配布権、日本語品質、機材とRAM/VRAMを測る条件。未確認ならスタブ継続。外部LLMへの自動fallbackは認めない。
3. **外部連携の説明と強制可能な権限**：送信項目・送り先・処理/保存/運用先・保持・residencyの版付き開示、誰が有効化できるか。提供元内のpush/送信/削除を制限できない場合の能力表示と利用範囲。初期オフでも所在地未確認だけで一律禁止しない。
4. **国内境界の証拠**：管理端末/hostの所在、依存のtelemetry・push・モデル取得・OS/driver/IME・backupの全通信経路、遮断と観測範囲。端末持出し・国外端末・外部メールは今回の連携許可で解決済みにしない。
5. **採用前の実機試験**：3OSのIME、複数メッセージ選択、仮想履歴、NVDA/VoiceOver/Orca。公式対応・AccessKit採用・他社採用では代替しない。GPUI保留解除は独立の根拠と試験が必要。
6. **将来リージョン**：今回の実装範囲へ広げず、既存data/backup移行・契約・運用アクセスの判断を将来に残す。地域選択で法適合を保証しない。
7. **文書の固定単位**：本文だけでなく、採用判断を支える担当原稿・一次情報証拠をどの版/hashに結び付けるか。DiscordのM-01を反映する版と出典IDも次版で決める。

## 検証の範囲

単独の市場担当レビューであり、別のレビュアーや外部モデルの独立レビューを実施したとはしない。curl、公開X読取り、公式資料の確認、ローカルの読取検査のみ。本文への修正提案は全て未適用で、本ファイルだけを新規作成した。
