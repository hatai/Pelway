# Issue #381 の作業ログ

## 実装者メモ（round 1）

対象 head: 33bf440923866b53cdbe506006991cd83d3bd394

1. 詰まった箇所: `crates/pelway-client-ui/tests/demo_banner.rs`: 「あなただけに表示」の行と msg3 以降の要素が `ElementHandle` の探索で見つからず、`if :` の条件付き要素の後ろの兄弟が落ちる制約と誤診して構造を直しかけた。`i-slint-backend-testing` の走査は `ItemRc::is_visible()` を通り、Flickable のビューポート外にクリップされた行を除外するのが原因だった（窓を 2400px 高にして全行を可視にして解決）。
2. 曖昧だった文: 「全画面に出す常時表示「デモ・模擬応答」をネイティブクライアントの骨組みに付け、人と AI の識別を読み上げ契約どおりにそろえる。」（Issue #381 本文「目的」）。「読み上げ契約どおり」の読み上げ全文の適用範囲が分からず、バッジの読み上げ名（AIメンバー）までをこの Issue の範囲と解釈した。
3. 仮定と解釈: 常時バナーは画面ごとの複製ではなく MainWindow 直下の帯として 1 つ配置した（show-today の切替で消えず「全画面」の契約を満たす最小形）。選ばなかった解釈: TodayView/ChannelView それぞれの内側に置く形（画面ごとに同じ帯を複製し、読み上げにも重複が出うるため）。
4. 足りなかった情報・検証手段: Slint テスト backend が「見つからない」要素を拾わない理由（visibility フィルタ）は crate ソースを読むまで分からず、`docs/` や既存テストのコメントにも記述が無かった。msg3 以降が落ちる症状は条件 `if` と位置が重なっていたため切り分けに debug example（使い捨て、削除済み）を使った。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「よくある失敗」相当の箇所に「Slint のテスト用 backend で要素が見つからないときは `ItemRc::is_visible()` の除外（スクロール外クリップ）を先に疑い、窓を縦長にしてから構造上の仮説を立てる」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: 03013469aaea6ea56290acfaaa28cbafc4790543

1. 詰まった箇所: `crates/pelway-client-ui/tests/demo_banner.rs` の折り返し検査が CI だけ落ちた。文字の自然幅はフォント計測に依存するため、固定幅（140px まで）は CI では折り返しに届かなかった。窓を段階的に狭めて必ず折り返すまで確かめる形に直した。さらに must-fix で「360px の固定検査が無い・`max-lines`+`clip` で文言が落ちても高さの assert が常に真」を指摘され、実装から行数上限を外して文言が必ず全部出る形に変えた。
2. 曖昧だった文: 「幅を 360px 相当まで狭めても『（模擬）』/ "simulated" が省略されず、最大 2 行に折り返されて全体が見える → 幅を変えてテキストが欠けないこと」（Issue #381 受け入れ条件）。「最大 2 行」を実装側の行数上限（max-lines）と読むか、テストが確かめる結果（360px で 2 行以内に収まる）と読むかが分からず、前者を取ったところ文言が clip で落ちうる形になった。
3. 仮定と解釈: 「テキストが欠けないこと」を優先し、行数上限を実装に持たせない解釈を取った（文言は行を増やして全部出る。360px で 2 行以内に収まることはテストで確かめる）。選ばなかった解釈: max-lines を残して「必要行数が 2 以下」を別手段で示す形（要素の高さは上限で頭打ちになるため、必要行数を直接には測れない）。
4. 足りなかった情報・検証手段: フォント計測が CI とローカルでどれくらい違うかを示すものが無く、固定幅の検査が CI だけ落ちる理由を CI のログとローカルの計測（text=58..171px で折り返す）を突き合わせて絞った。テスト用 backend の `size()` が割り当て幅を返すこと（自然幅ではない）も実測で確認した。
5. 改善案: `docs/agents/` のレビュー・実装のどちらかの手順に「テキストの折り返し・切れを検査するテストは固定幅ではなく、文言の計測値か段階的な縮小で折り返しが必ず起きる幅まで確かめる（フォント計測は環境差がある）」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 2 追記・main 取り込み）

対象 head: 61f6eb5fa60c9fed7d8db3860b939b390c9d700b（マージコミットのため署名なし）

1. 詰まった箇所: main に PR #445（Issue #379）が先に入り、`main.slint`・`.ftl`・`i18n.rs`・`slint_window.rs` が衝突。`AiBadgeView` が意味的にも衝突した: こちらはバッジの内側の文字に `accessible-label: label-a11y`（「AIメンバー」/"AI member"）を出す設計、main は `accessible-role: none` + 空ラベルでバッジを読み上げから外し、呼称「AI Bot」を行側のラベルが言う設計。
2. 曖昧だった文: 「見える『AI』は読み上げに二重に載せない。種類の呼称は #323 の『AI Bot』」（Issue #381 背景・前提）と「読み上げ属性の全体適用（#379）。本 Issue はバナーと識別の表示契約に限る」（対象外）。round 1 では「バッジに独自の読み上げ名を付ける」が二重でないと読んだが、#379 着地後は行ラベル「AI Bot」＋バッジ「AIメンバー」の 2 読みになり、呼称も「AI Bot」と食い違う。
3. 仮定と解釈: 既存コード優先（AGENTS.md「Code over docs」）で main の契約を採用し、`label-a11y` 機構と `label-ai-member-a11y` キーを全撤去した（「AIメンバー」は「AI Bot」と別名を導入する死にキーになるため）。バッジの可視表示・人の行に出さない条件は変えない。選ばなかった解釈: `label-a11y` を残して main の `accessible-role: none` を外す形（#379 の着地を戻し、行ラベルとバッジの二重読みを復活させてしまう）。
4. 足りなかった情報・検証手段: レビュー済み設計が先にマージされた別 Issue の契約と意味的に衝突したときの手順が implement-issue に無く、今回は Issue 本文と着地コードの両方が同じ方向を示したため自分で判断できた。片方しか示さない場合は人に聞く必要がある。
5. 改善案: implement-issue の「よくある失敗」に「レビュー後に main が動いて設計が着地済み契約と衝突したら、Issue 本文＋着地コードの両方で新契約を確認し、自分の差分（キー・プロパティ・テスト）まで遡って撤去する。テストは旧契約を固定したままにしない」を足す。

使ったスキル: implement-issue、fix-merge-conflicts

round 2 追記への補足（CI 失敗の修正）: `demo_banner.rs` の行・バッジ検査は窓を 800x2400 に固定していたが、CI のフォント計測では内容がビューポートに収まらず、en の判断カードのバッジが Flickable のクリップで探索から落ちた（`test`・`octocov`・`mise run verify` すべて同じ assert で失敗）。可視になるまで窓を段階的に伸ばす `find_growing` に全検査を揃え、フォント環境差に依存しない形にした。

round 2 追記への補足2（CI 失敗の根本原因・確定）: en の判断カードのバッジ消失は 2 つの独立した環境差が重なったものだった。

1. 条件付き要素の実体化: `if data-decision-visible` の中に実体化されるカードで、内側 `HorizontalLayout` の直接の子にある `if`/`for`（条件が転送バインディング `data-requester-is-ai` を読む）は CI の en で子要素を作らなかった。`if true`（定数条件）は同じ位置で出る、同じ条件でもカード直下の `VerticalLayout` の子なら出ることを `DIAG` 要素で突き止め、依頼者行を AI/人の 2 バリアントに分けてバッジをレイアウト内の非条件要素にした。
2. Flickable の水平クリップ: テストの探索は `ItemRc::is_visible()` を通るためクリップ外は見えない。窓 800px ではチャンネル列の `Flickable` が 340px しかなく（右端 x=596）、en ではタイトル `Decision request` が長い分だけ依頼者行が右に寄り、バッジが右端クリップを超えて不可視になっていた（静止要素の消失＝クリップで確定）。`find_growing` の窓幅を 1600px にして解消。

bridge.rs もデータ確定後に `data-decision-visible` を立てる順に変えた（実体化時に `if` が初期値で評価される余地を減らす防御）。診断用の `DIAG` 出力・`Text "X"`・`Rectangle` ラップ・lang 順反転は撤去済み。

## 実装者メモ（round 3）

対象 head: a9be43edf40b038c20c9bee88b8ab724795ed726

1. 詰まった箇所: Claude の must-fix で「`AiBadgeView` の `horizontal-stretch: 0` では横に伸びるのを止められない」と指摘された。Slint 1.18.1 の `adjust_items` は行の全要素が stretch 0 のとき全員を stretch 1 とみなして余りを等分するため、バッジが帯状に伸びて「名前の直後」から外れる。レビュー担当は実行しない決まりで i-slint-core のソースから導いた指摘で、ローカルで要素の実ジオメトリ（レール行のダンプ）を取って確認した。
2. 曖昧だった文: 「余りを受ける要素を 1 つ決めて `horizontal-stretch: 1` を付ける（右寄せの在席・待ち時間・時刻の Text、枠の LinkText など）。末尾に受け手が無いバナーと PrivateNoteRow は、空の `Rectangle { horizontal-stretch: 1; }` を足すか、`alignment: start` にする」。どの要素が受け手になるかは候補が挙がっていたので、指摘どおり右寄せ要素に `horizontal-stretch: 1`、受け手が無い行は空の Rectangle を足す形にした。
3. 仮定と解釈: 「バッジの左端が『名前の右端＋spacing』にあること」は、行内でバッジと縦に重なる要素のうち右端が最もバッジに近いものとの差分が spacing になることと読んだ。最初は縦の重なりを見ずに「行内で最も右にある要素」としていたら、別の行の要素の右端を直前と取り違えて失敗した（`assert_badge_packed` で縦の重なりフィルタを足して直した）。判断カードの人バリアント行（バッジ無し）にも同じ等分配の欠陥があるため同じ形で受け手を置いた。
4. 足りなかった情報・検証手段: Slint のレイアウトで「全員 stretch 0 → 等分配」になる挙動は crate の `layout.rs` を読まないと分からず、ドキュメントにも無かった。`ElementHandle` には `text()` が無いため、テキストの位置確認は `accessible_label`（Text の場合は text と一致）か位置の突き合わせになる。
5. 改善案: 「よくある失敗」に「Slint の HorizontalLayout で stretch の受け手が 1 つも無いと余りが等分に配られ、stretch 0 の要素も帯状に伸びる。行の末尾に右寄せ要素か空の `Rectangle { horizontal-stretch: 1 }` を置く」を足す。

使ったスキル: implement-issue

round 3 追記（main の時系列仮想化との統合と CI フレーク）: main に PR #446（Issue #385。時系列を `timeline-items` モデル駆動の ListView 仮想化）が入り `main.slint` と衝突。main の `for item[i] in root.timeline-items` 構造を採用し、ループ内の `DecisionCardView`・`PrivateNoteRow` に `tr-ai-badge` の束縛とテスト用 id（`decision-card`・`private-note` の Rectangle 枠）を戻した。`demo_banner.rs` は消えた `set_data_msg*/note_*` プロパティを `TimelineItem` モデル構築に置き換え（`accessibility.rs` と同型）。

統合後の CI で `timeline_virtualization`（Issue #385 側のテスト）だけが失敗: 実体化行は 6 行あるのに記録行番号が 0 件。`record-timeline-row` の結線とスクロール機構は同じ CI 実行内の他テスト（32 回スクロールして全行記録を要求する `every_row_kind` など）で動いていたため、「2 回目の小さいスクロール（-40px）が changed を 1 件も起こさない」境界条件と判断した。小スクロールで保持行が必ず再配置されて再記録されるという仮定はテストに不要なので、画面ぶん以上（-900px）めくって実体化セットをまるごと入れ替える形に直した（新規実体化・再利用のどちらでも全行が記録される）。
