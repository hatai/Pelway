# Issue #331 の作業ログ

## 実装者メモ（round 1）

対象 head: ecab2c3b8d66b9401301ffd086487b34cb2855a0（PR-1 実装コミット、署名付け替え後）

1. 詰まった箇所: 手順 6（pr-size）: 単一 PR だと追加 +1442 が上限 800 を超え、Issue コメントで分割案を提示し `needs-human` で止まった。指揮役が split-plan（3 PR 積み上げ）を承認したため、PR-1 = client-core のみ（+313）で再開。ほか、Slint で `if` 直下の `for` が書けず三項で回避、i-slint-backend-testing が複数 `MainWindow` で落ちるため窓検査を 1 テストに集約（後続 PR の話）。
2. 曖昧だった文: 「読み取り側は既存の inbox.rs の定義（自分の判断待ち／自分の依頼の2区分）が正本。main.slint の TodayView モック（3区分）とは食い違うので、コードの定義に合わせる。」（Issue #331 本文「背景・前提」）— TodayView 全体を 2 区分に作り替えるか、判断待ちだけを 2 区分に見せるか。
3. 仮定と解釈: `TodaySectionKind` の 3 区分を消して正本の 2 区分に作り替える解釈を採用（3 区分を残すと「コードの定義に合わせる」と矛盾）。PR-1 では `DecisionBoard` を UI 非依存の台帳窓口とし、HTTP や永続化は持たせず「将来送る `DecideRequest` を組み立てる」責務に限定した。
4. 足りなかった情報・検証手段: `mise run verify` がこの sandbox では直接動かず（cargo shim が mbx 経由で `~/.cache/mbx` read-only、`/etc/resolv.conf` が `/mnt/wsl/resolv.conf` へ dangling で rustup sync が失敗）、rustup toolchain 直指定 + `MBX_CACHE_DIR=/tmp/mbx-cache` + `MISE_DISABLE_TOOLS=rust,node` + deny-check のみ DNS 修正済み userns の分割実行で同等を確認。
5. 改善案: `docs/dev-environment.md` の「ローカル検証」に「pelway-sandbox では `MISE_DISABLE_TOOLS=rust` + rustup toolchain 直指定 + `MBX_CACHE_DIR` で verify を分割実行する」旨を足す。

使ったスキル: implement-issue。TDD（seam: `pelway-client-core::decision::DecisionBoard`）。

## 実装者メモ（PR-2 round 1）

対象 head: 2d21f470ddab2a4bb7d2ad264c5b30d7c04cb8de（PR-2 実装コミット、署名付け替え後）

1. 詰まった箇所: `main.slint` で `if` 直下に `for` を書けず（文法エラー）、選択肢の列は `root.data-decided ? [] : root.data-options` の三項で空配列に切り替えて回避。`i18n_keys` の `.slint` 走査が `tr-<key>` 形のプロパティ名を Fluent キーとみなすため、コンポーネント内 prop 名も ftl キーと同じフル名（`tr-decision-choose` 等）に揃えた。`no_color_values` の走査がコメント中の `#331` を色値と誤検知するため `Issue 331` 表記に統一。
2. 曖昧だった文: 「読み取り側は既存の inbox.rs の定義（自分の判断待ち／自分の依頼の2区分）が正本」（Issue #331 本文「背景・前提」）— 3 区分モックを残したまま判断待ちだけ 2 区分にするか、TodayView 自体を 2 区分に作り替えるか。
3. 仮定と解釈: `TodaySectionKind` の 3 区分を消して正本の 2 区分（`NeedsMyDecision`/`MyRequests`、キー `today-section-mine`/`today-section-requests`）に作り替える解釈を採用。待ち時間は `format_wait_ms`（ja: 1分/1時間5分/1日2時間、en: 1m/1h5m、0 は 1 分に切り上げ）で一覧表示用に整形し、確定後のカードは消さず確定内容・決定者・版に表示を切り替える。
4. 足りなかった情報・検証手段: i-slint-backend-testing / winit は 1 プロセスに複数 `MainWindow` を作れず、窓を触る結合検査は PR-3 の 1 テストへの集約で対処（PR-2 は窓を作らない検査のみ）。
5. 改善案: `docs/dev-environment.md` の client-ui 向け節に「Slint の `if` 直下の `for` は不可・`tr-*` prop 名は ftl キーと同名にする・`#数字` は色値走査に誤検知される」を足す。

使ったスキル: implement-issue。

## 実装者メモ（PR-3 round 1）

対象 head: 7177046f7300e19aa853c91e4bc11e76afdbd06b（PR-3 実装コミット、rebase・署名付け替え後）

1. 詰まった箇所: i-slint-backend-testing / winit が 1 プロセスに複数 `MainWindow` を作れず（`EventLoop can't be recreated` / `No backends configured`）、窓を触る結合検査を `pending_screen_end_to_end` 1 テストに集約し、行の値だけの検査は窓なしのテストに分けた。`window.as_weak()` は `slint::ComponentHandle` の import が要る。
2. 曖昧だった文: 「項目から対象のチャットへ戻れる」（Issue #331 本文「受け入れ条件」）— 画面遷移の厳密な動作（どの状態が切り替わるか）の定義が本文にない。
3. 仮定と解釈: 「戻れる」は「Today 画面から判断待ちの対象チャンネルの表示へ切り替わる」ことと解釈し、`open-decision-chat` callback が `decision_id` → チャンネル解決 → 画面切り替えを行い、操作試験で `view` プロパティの遷移を検証。`DecisionShell` は一覧値を台帳から毎回組み立てるため、確定後の再適用で項目が復活しないことをそのまま検証対象にした。
4. 足りなかった情報・検証手段: メンバー・チャンネルの ID → 表示名の正本が無いため、デモ用に `DecisionShell::sample` が合成データ（ユイ・濱田・リク・# 案件B）を持つ構成にした。本番の名前解決は将来の inbox 読み取り層の役割。
5. 改善案: `GLOSSARY.md` に「判断待ちの一覧項目は台帳から毎回組み立て、確定後は一覧から消えてチャット内カードだけが残る」を足す。

使ったスキル: implement-issue。

## 実装者メモ（PR-3 round 2）

対象 head: cb2a80eb1c86152dfd2a6dd498a8b23dfdab2584（must-fix 修正コミット、署名付け替え後）

1. 詰まった箇所: レビュー判定が Codex approve / Claude request-changes に割れ、旧版の「割れたら停止」に従ってエスカレーションしたが、恒久指示（修正優先）で修正側を正として round 2 に入った。`open-decision-chat` の遷移で `channel_name_of` の結果を捨てていた欠陥は、`DecisionShell::open_chat`（遷移先の解決と `channel_card` の指し替えを一度に行う）に集約して直した。
2. 曖昧だった文: 「項目から対象のチャットへ戻れる」（Issue #331 本文「受け入れ条件」）— 「対象の」は「その判断待ちの属するチャンネルへ、題名と中身が切り替わる」ことと読み替えた（レビュー指摘の直し方に合意）。
3. 仮定と解釈: 「チャット内の判断カードでも同じ決定ができる」の検証は、callback が一覧・カード共通であっても、カードの props（`data-decision-id`/`data-decision-version`）を読んで送る形にしないと `apply_decision_screen` の値の間違いを拾えないと解釈し、テストをカード props 経由の決定に変えた。
4. 足りなかった情報・検証手段: 「対象のチャット」に遷移先の題名が出ることを確かめる入口（`data-channel-title` の設定側）が初期実装で欠けていた。レビューが指摘するまで `set_show_today(false)` だけで足りると考えていた。
5. 改善案: `.agents/skills/implement-issue` の手順 9「判定が割れたら停止」の後段に「恒久指示（2026-11-16）: 要修正と承認で割れたときは修正を優先する」を追記する。

使ったスキル: implement-issue。
