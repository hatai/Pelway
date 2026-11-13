# Issue #227 の作業ログ

## 対象と方針

Slint Royalty-free License 2.0 の条件を一次資料（slint-ui/slint v1.18.1 の `LICENSE.md`・`FAQ.md`・`LICENSES/LicenseRef-Slint-Royalty-free-2.0.md`・AboutSlint docs）から `docs/research/slint-royalty-free-license.md` に表と引用で記録し、A1 行から参照した。クライアントコードは変更せず、未実装の attribution 表示は #237 に切り出した。

## 変更したファイル

- `docs/research/slint-royalty-free-license.md`（新規）
- `docs/research/README.md`
- `docs/technical/v0.7/body/08-decisions-open.md`
- `docs/worklogs/issue-227.md`（本ファイル）

## 判断

- フォロー Issue は attribution 表示だけに絞った。根拠は、`ui/main.slint` に `AboutSlint`・`MadeWithSlint`・Splash Screen・トップレベルメニューが無く、`Cargo.toml` が表示場所を未決としているため。
- #237 の方式はアプリ内 `AboutSlint`（トップレベルメニューから About 画面/ダイアログ）を採用し、公開ページ badge は配布ページ未定のため対象外にした。

## 検証

- `scripts/agent-gh dev api repos/slint-ui/slint/contents/...` で Slint v1.18.1 のライセンス関連ファイルを取得（exit 0）
- `mise run verify` → exit 0（2026-11-13 JST）

## 実装者メモ（round 1）

1. 詰まった箇所: `docs/technical/v0.7/body/08-decisions-open.md`: A1 の行が「継承する技術選定」表の 38 行目にあり、最初の読み取りでは見出し付近しか確認していなかった。`grep A1|Slint` で行を特定して編集した。
2. 曖昧だった文: 「条件のうち、今のクライアント（`crates/pelway-client-ui`）が満たしていないもの（例: 表示が無い）を、フォロー Issue として起票する」（Issue #227「受け入れ条件」）
3. 仮定と解釈: 「満たしていないもの」は配布時に必須になる attribution 表示が未実装であることと解釈し、#237 を attribution 実装に限定した。Embedded Systems・API 公開・license notices は現状のデスクトップアプリ骨組みでは問題にならないと判断し、フォロー Issue には含めなかった。
4. 足りなかった情報・検証手段: `scripts/agent-gh dev issue view 237` が 30 秒でタイムアウトしたため、#237 の本文は作成時の heredoc を根拠にした。`mise run verify` は exit 0 で完了したが、出力は長く末尾まで目視していない。
5. 改善案: `implement-issue` の手順 7 に「フォロー Issue を作った場合は、Issue 番号を `docs/research/` の文書と A1 行の両方へ書く」を足す。
