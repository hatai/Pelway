# Pelway技術文書 v0.6 最終確認・完成判定

2026-11-08 23:49 JST。文書として完成。これは設計案・未決・有効化条件をそろえた文書の完成であり、実装・試験・実接続・支出の許可、安全性/国内適合/ライセンスの検証完了ではない。S1は未解決で、実外部I/Oは無効のまま。案A〜Cと効力発生点は未選択。

## 最終賛否と確認範囲

| 担当/確認者 | 判定と根拠 |
|---|---|
| 市場 | [v0.5は賛成](fixed/pelway-technical-docs/reviews/market-v0.5.md)。v0.6はREADME/03/08だけの補正で、市場範囲の証拠・許諾は無変更。全体再依頼はしていない |
| プロダクト | [v0.5賛成](fixed/pelway-technical-docs/reviews/product-v0.5.md)に加え、[v0.6の08補正も賛成](review-records/product-v0.6-delta.md)。残る必須修正なし |
| 技術 | [v0.5賛成](fixed/pelway-technical-docs/reviews/technical-v0.5.md)に加え、[v0.6の08補正も賛成](review-records/technical-v0.6-delta.md)。残る必須修正なし |
| UI | [v0.5賛成](fixed/pelway-technical-docs/reviews/ui-v0.5.md)に加え、[v0.6のキー転記も賛成](review-records/ui-v0.6-delta.md)。残る必須修正なし。外観変更なし |
| Codex | [v0.5文書賛成](fixed/pelway-technical-docs/reviews/codex-v0.5.md)、[D5-01〜06差分も解消](review-records/codex-v0.6-delta.md)。exit0、17.180秒。S1現方式の採用に反対し、未解決としての文書記録に賛成 |
| Claude | [v0.5はD5条件付き](fixed/pelway-technical-docs/reviews/claude-v0.5.md)。[v0.6で内容は解消](review-records/claude-v0.6-delta.md)、exit0、48.483秒。残った参照表記は[証拠コピーの補足確認で閉じた](review-records/claude-v0.6-clarification.md)、exit0、16.819秒。指定された最終表記を下記採否表と注記に適用済み。追加レビュー不要 |

全原稿を実際に受領・読取した。Codex/Claudeは提示資料の読取評価で、静的検査の独立実行はしていない。v0.5以降に同じ固定本文の全体レビューを繰り返さず、D5の補正箇所と参照1点だけを確認した。

## 正本の読み分け

本文12件は[固定本文](fixed/pelway-technical-docs/README.md)。固定時の「候補」「確認待ち」という進行状態は当時の記録で、現在の完成判定はこの最終記録を優先する。

採否の最終記録は[最終採否表](review-records/disposition-v0.6.md)と[参照表記の付随注記](review-records/closure-v0.6.md)。採否表のD5-05行だけを編集上の訂正として新しい最終記録に反映し、fixed配下の本文・証拠コピーは上書きしていない。固定v0.5のV3-04行はそのまま。新v0.6の別コピーに内訳参照を加えた[証拠差分](review-records/v0.5-to-v0.6-evidence.diff)を同梱する。

## 実装前の未決・阻害

- S1-J1〜J3：権限取消の効力発生点、要求/完了/実送出の境界、上限時間・超過時の責任者と表示。要求単位の失効強制と検査後の後発送出拒否を実現する具体方式と証拠要件。送出口の失効は外部run停止の証拠ではない。
- 提供元の失効・冪等key・結果照会の仕様、DB永続commit/backup復旧と照合、国内の運用/通信経路、機材/モデル、Slint許諾・rmcp残存MIT・推移依存/NOTICE/SBOM、3 OS実機試験、正式仕様の判断。
- T6の期待値、短い返信の完全一致条件、結果不明の手動終結、承認の引継ぎ。結果不明を新ID/新承認で回避して再送しない。
- [取消完了後のUI文言案](review-records/ui-completion-decision-input.md)はJ1に依存するJ3の判断材料で、未採用。完成条件に追加しない。

実外部I/O有効化は、[08](fixed/pelway-technical-docs/08-decisions-open.md)の正式判断・証明/検証と既存ゲート・個別許可を満たしてから。既存計画、工数、合成データだけの局所試作は変更していない。需要・実装・安全ゲートは通過扱いにしない。

## 静的検査の範囲

固定本文12、凍結時依存コピー112、生成参照案内4のハッシュ、v0.1〜v0.5旧固定版、保護原本15件を確認。[静的記録](verification/static-v0.6.json)と[本文manifest](verification/v0.6-manifest.json)・[依存manifest](verification/v0.6-dependencies-manifest.json)を同梱。相対ファイルリンクと名称衝突の実見出しは存在検査を行い、全出典の事実確認や実行時/法的適合の検証とは区別する。納品フォルダ全体の点検結果はverification/delivery-static.jsonへ記録する。ZIPは作成後にCRCと格納内容一致を検査し、合格したものだけ納品する。
